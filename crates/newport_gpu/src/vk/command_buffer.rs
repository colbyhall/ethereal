use super::{
	Buffer,
	Device,
	DeviceThreadInfo,
	Pipeline,
	RenderPass,
	Texture,
};
use crate::{
	Layout,
	PipelineDescription,
};

use newport_math::{
	Color,
	Rect,
};

use ash::version::DeviceV1_0;
use ash::vk;

use std::slice::{
	from_raw_parts,
	from_ref,
};
use std::sync::Arc;

pub struct GraphicsCommandBuffer {
	pub owner: Arc<Device>,

	pub command_buffer: vk::CommandBuffer,

	pub framebuffers: Vec<vk::Framebuffer>,
	pub pipelines: Vec<Arc<Pipeline>>,
	pub textures: Vec<Arc<Texture>>,
	pub buffers: Vec<Arc<Buffer>>,

	pub current_scissor: Option<Rect>,
	pub current_attachments: Option<Vec<Arc<Texture>>>,
}

fn layout_to_image_layout(layout: Layout) -> vk::ImageLayout {
	match layout {
		Layout::Undefined => vk::ImageLayout::UNDEFINED,
		Layout::General => vk::ImageLayout::GENERAL,
		Layout::ColorAttachment => vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
		Layout::DepthAttachment => vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
		Layout::TransferSrc => vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
		Layout::TransferDst => vk::ImageLayout::TRANSFER_DST_OPTIMAL,
		Layout::ShaderReadOnly => vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
		Layout::Present => vk::ImageLayout::PRESENT_SRC_KHR,
	}
}

impl GraphicsCommandBuffer {
	pub fn begin(&mut self) {
		unsafe {
			self.owner
				.logical
				.reset_command_buffer(self.command_buffer, vk::CommandBufferResetFlags::default())
				.unwrap()
		};

		let begin_info = vk::CommandBufferBeginInfo::builder()
			.flags(vk::CommandBufferUsageFlags::SIMULTANEOUS_USE);

		unsafe {
			self.owner
				.logical
				.begin_command_buffer(self.command_buffer, &begin_info)
				.unwrap()
		};
	}

	pub fn end(&mut self) {
		unsafe {
			self.owner
				.logical
				.end_command_buffer(self.command_buffer)
				.unwrap()
		};
	}

	pub fn copy_buffer_to_texture(&mut self, dst: Arc<Texture>, src: Arc<Buffer>) {
		let subresource = vk::ImageSubresourceLayers::builder()
			.aspect_mask(vk::ImageAspectFlags::COLOR)
			.layer_count(1);

		let extent = vk::Extent3D::builder()
			.width(dst.width)
			.height(dst.height)
			.depth(dst.depth);

		let region = vk::BufferImageCopy::builder()
			.image_subresource(subresource.build())
			.image_extent(extent.build());

		unsafe {
			self.owner.logical.cmd_copy_buffer_to_image(
				self.command_buffer,
				src.handle,
				dst.image,
				vk::ImageLayout::TRANSFER_DST_OPTIMAL,
				&[region.build()],
			)
		};
	}

	pub fn copy_buffer_to_buffer(&mut self, dst: Arc<Buffer>, src: Arc<Buffer>) {
		assert_eq!(dst.size, src.size);

		let region = vk::BufferCopy::builder().size(dst.size as u64).build();

		unsafe {
			self.owner.logical.cmd_copy_buffer(
				self.command_buffer,
				src.handle,
				dst.handle,
				&[region],
			)
		};
	}

	pub fn resource_barrier_texture(
		&mut self,
		texture: Arc<Texture>,
		old_layout: Layout,
		new_layout: Layout,
	) {
		let mut barrier = vk::ImageMemoryBarrier::builder()
			.old_layout(layout_to_image_layout(old_layout))
			.new_layout(layout_to_image_layout(new_layout))
			.image(texture.image)
			.src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
			.dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED);

		// TODO: Mips
		barrier = barrier.subresource_range(
			vk::ImageSubresourceRange::builder()
				.aspect_mask(vk::ImageAspectFlags::COLOR)
				.base_mip_level(0)
				.level_count(1)
				.base_array_layer(0)
				.layer_count(1)
				.build(),
		);

		let src_stage;
		let dst_stage;
		match (old_layout, new_layout) {
			(Layout::Undefined, Layout::TransferDst) => {
				barrier = barrier.dst_access_mask(vk::AccessFlags::TRANSFER_WRITE);

				src_stage = vk::PipelineStageFlags::TOP_OF_PIPE;
				dst_stage = vk::PipelineStageFlags::TRANSFER;
			}
			(Layout::TransferDst, Layout::ShaderReadOnly) => {
				barrier = barrier
					.src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
					.dst_access_mask(vk::AccessFlags::SHADER_READ);

				src_stage = vk::PipelineStageFlags::TRANSFER;
				dst_stage = vk::PipelineStageFlags::FRAGMENT_SHADER;
			}
			(Layout::ColorAttachment, Layout::ShaderReadOnly) => {
				src_stage = vk::PipelineStageFlags::BOTTOM_OF_PIPE;
				dst_stage = vk::PipelineStageFlags::BOTTOM_OF_PIPE;
			}
			(Layout::ColorAttachment, Layout::Present) => {
				src_stage = vk::PipelineStageFlags::BOTTOM_OF_PIPE;
				dst_stage = vk::PipelineStageFlags::BOTTOM_OF_PIPE;
			}
			(Layout::Undefined, Layout::Present) => {
				src_stage = vk::PipelineStageFlags::BOTTOM_OF_PIPE;
				dst_stage = vk::PipelineStageFlags::BOTTOM_OF_PIPE;
			}
			_ => unimplemented!(),
		}

		unsafe {
			self.owner.logical.cmd_pipeline_barrier(
				self.command_buffer,
				src_stage,
				dst_stage,
				vk::DependencyFlags::default(),
				&[],
				&[],
				&[barrier.build()],
			)
		};
	}
}

impl GraphicsCommandBuffer {
	pub fn new(owner: Arc<Device>) -> Result<GraphicsCommandBuffer, ()> {
		let handle = {
			let mut thread_infos = owner.thread_info.lock().unwrap();
			let thread_id = std::thread::current().id();

			let mut thread_info = thread_infos.get_mut(&thread_id);
			if thread_info.is_none() {
				thread_infos.insert(thread_id, DeviceThreadInfo::default());
				thread_info = thread_infos.get_mut(&thread_id)
			}
			let thread_info = thread_info.unwrap();

			if thread_info.graphics_pool == vk::CommandPool::default() {
				let create_info = vk::CommandPoolCreateInfo::builder()
					.flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
					.queue_family_index(owner.graphics_family_index.unwrap());

				thread_info.graphics_pool = unsafe {
					owner
						.logical
						.create_command_pool(&create_info, None)
						.unwrap()
				};
			}

			let alloc_info = vk::CommandBufferAllocateInfo::builder()
				.command_pool(thread_info.graphics_pool)
				.level(vk::CommandBufferLevel::PRIMARY)
				.command_buffer_count(1);

			let handle = unsafe { owner.logical.allocate_command_buffers(&alloc_info) };
			if handle.is_err() {
				return Err(());
			}
			handle.unwrap()[0]
		};

		Ok(GraphicsCommandBuffer {
			owner: owner,

			command_buffer: handle,

			framebuffers: Vec::new(),
			pipelines: Vec::new(),
			textures: Vec::new(),
			buffers: Vec::new(),

			current_scissor: None,
			current_attachments: None,
		})
	}

	pub fn begin_render_pass(
		&mut self,
		render_pass: Arc<RenderPass>,
		attachments: &[Arc<Texture>],
	) {
		let extent = vk::Extent2D::builder()
			.width(attachments[0].width)
			.height(attachments[0].height)
			.build();

		let render_pass_handle = render_pass.handle;

		for it in attachments.iter() {
			self.textures.push(it.clone());
		}
		self.current_attachments = Some(attachments.to_vec()); // TODO: Temp Allocator

		// Make the framebuffer
		let mut views = Vec::with_capacity(attachments.len()); // TODO: Temp Allocator
		for it in attachments.iter() {
			views.push(it.view);
		}

		let create_info = vk::FramebufferCreateInfo::builder()
			.render_pass(render_pass_handle)
			.attachments(&views[..])
			.width(extent.width)
			.height(extent.height)
			.layers(1);

		let framebuffer = unsafe {
			self.owner
				.logical
				.create_framebuffer(&create_info, None)
				.unwrap()
		};
		self.framebuffers.push(framebuffer);

		let render_area = vk::Rect2D::builder().extent(extent);

		let begin_info = vk::RenderPassBeginInfo::builder()
			.render_pass(render_pass_handle)
			.framebuffer(framebuffer)
			.render_area(render_area.build());

		unsafe {
			self.owner.logical.cmd_begin_render_pass(
				self.command_buffer,
				&begin_info,
				vk::SubpassContents::INLINE,
			)
		};
	}

	pub fn end_render_pass(&mut self) {
		unsafe { self.owner.logical.cmd_end_render_pass(self.command_buffer) };
		self.current_scissor = None;
		self.current_attachments = None;
	}

	pub fn bind_scissor(&mut self, scissor: Option<Rect>) {
		self.current_scissor = scissor;
	}

	pub fn bind_pipeline(&mut self, pipeline: Arc<Pipeline>) {
		unsafe {
			self.owner.logical.cmd_bind_pipeline(
				self.command_buffer,
				vk::PipelineBindPoint::GRAPHICS,
				pipeline.handle,
			);
			self.owner.logical.cmd_bind_descriptor_sets(
				self.command_buffer,
				vk::PipelineBindPoint::GRAPHICS,
				pipeline.layout,
				0,
				&[pipeline.owner.bindless_set],
				&[],
			);

			let viewport = vk::Viewport::builder()
				.width(self.textures.last().unwrap().width as f32)
				.height(self.textures.last().unwrap().height as f32)
				.max_depth(1.0);
			self.owner
				.logical
				.cmd_set_viewport(self.command_buffer, 0, from_ref(&viewport));

			if self.current_scissor.is_some() {
				let scissor = self.current_scissor.unwrap();

				let size = scissor.size();
				let rect = vk::Rect2D::builder()
					.offset(
						vk::Offset2D::builder()
							.x(scissor.min.x as i32)
							.y(scissor.min.y as i32)
							.build(),
					)
					.extent(
						vk::Extent2D::builder()
							.width(size.x as u32)
							.height(size.y as u32)
							.build(),
					);

				self.owner
					.logical
					.cmd_set_scissor(self.command_buffer, 0, from_ref(&rect));
			} else {
				let rect = vk::Rect2D::builder()
					.extent(
						vk::Extent2D::builder()
							.width(viewport.width as u32)
							.height(viewport.height as u32)
							.build(),
					)
					.build();

				self.owner
					.logical
					.cmd_set_scissor(self.command_buffer, 0, from_ref(&rect));
			}
		}

		self.pipelines.push(pipeline);
	}

	pub fn bind_vertex_buffer(&mut self, buffer: Arc<Buffer>) {
		let offset = 0;
		unsafe {
			self.owner.logical.cmd_bind_vertex_buffers(
				self.command_buffer,
				0,
				from_ref(&buffer.handle),
				from_ref(&offset),
			)
		};
		self.bind_buffer(buffer);
	}

	pub fn bind_index_buffer(&mut self, buffer: Arc<Buffer>) {
		let offset = 0;
		unsafe {
			self.owner.logical.cmd_bind_index_buffer(
				self.command_buffer,
				buffer.handle,
				offset,
				vk::IndexType::UINT32,
			)
		};
		self.bind_buffer(buffer);
	}

	pub fn bind_buffer(&mut self, buffer: Arc<Buffer>) {
		self.buffers.push(buffer);
	}

	pub fn bind_textures(&mut self, texture: Arc<Texture>) {
		self.textures.push(texture);
	}

	pub fn draw(&mut self, vertex_count: usize, first_vertex: usize) {
		unsafe {
			self.owner.logical.cmd_draw(
				self.command_buffer,
				vertex_count as u32,
				1,
				first_vertex as u32,
				0,
			)
		};
	}

	pub fn draw_indexed(&mut self, index_count: usize, first_index: usize) {
		unsafe {
			self.owner.logical.cmd_draw_indexed(
				self.command_buffer,
				index_count as u32,
				1,
				first_index as u32,
				0,
				0,
			)
		};
	}

	pub fn clear(&mut self, color: Color) {
		let attachments = self.current_attachments.as_ref().unwrap();
		assert!(attachments.len() > 0);

		let mut clear = Vec::with_capacity(attachments.len());
		for (index, _) in attachments.iter().enumerate() {
			let clear_value = vk::ClearValue {
				color: vk::ClearColorValue {
					float32: [color.r, color.g, color.b, color.a],
				},
			};

			clear.push(
				vk::ClearAttachment::builder()
					.aspect_mask(vk::ImageAspectFlags::COLOR)
					.color_attachment(index as u32)
					.clear_value(clear_value)
					.build(),
			);
		}

		let extent = vk::Extent2D::builder()
			.width(attachments[0].width)
			.height(attachments[0].height)
			.build();
		let clear_rect = vk::ClearRect::builder()
			.rect(vk::Rect2D::builder().extent(extent).build())
			.layer_count(1)
			.build();
		unsafe {
			self.owner
				.logical
				.cmd_clear_attachments(self.command_buffer, &clear[..], &[clear_rect])
		};
	}

	// TODO: Make this less all over the place
	pub fn push_constants(&mut self, t: &[u32]) {
		let pipeline = &self.pipelines.last().unwrap();

		let desc = match &pipeline.desc {
			PipelineDescription::Graphics(desc) => desc,
			_ => unreachable!(),
		};
		// assert_eq!(size_of::<u32>() * t.len(), desc.push_constant_size);

		unsafe {
			self.owner.logical.cmd_push_constants(
				self.command_buffer,
				pipeline.layout,
				vk::ShaderStageFlags::ALL_GRAPHICS,
				0,
				from_raw_parts(t.as_ptr() as *const u8, desc.push_constant_size),
			);
		}
	}
}

impl Drop for GraphicsCommandBuffer {
	fn drop(&mut self) {
		let thread_infos = self.owner.thread_info.lock().unwrap();
		let thread_id = std::thread::current().id();

		let thread_info = thread_infos.get(&thread_id).unwrap();

		unsafe {
			self.owner
				.logical
				.free_command_buffers(thread_info.graphics_pool, &[self.command_buffer]);
			self.framebuffers
				.iter()
				.for_each(|it| self.owner.logical.destroy_framebuffer(*it, None));
		}
	}
}
