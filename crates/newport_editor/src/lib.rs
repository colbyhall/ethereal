use newport_engine::{ Module, Engine, EngineBuilder, WindowEvent };
use newport_graphics::{ Graphics };
pub use newport_egui::*;
use newport_gpu as gpu;
use newport_os as os;

use std::sync::{ Mutex, MutexGuard };

mod menu_tab;
use menu_tab::*;

pub trait EditorPage {
    fn can_close(&self) -> bool {
        true
    }

    fn name(&self) -> &str;

    fn show(&self, ctx: &CtxRef);
}

struct HomePage;

impl EditorPage for HomePage {
    fn can_close(&self) -> bool {
        false
    }

    fn name(&self) -> &str {
        "Home"
    }

    fn show(&self, ctx: &CtxRef) {
        SidePanel::left("world", 300.0).show(ctx, |ui|{
            ui.heading("Hello World");

            ui.button("Buttons").clicked();
        });
    }
}

struct TestPage {
    name: String
}

impl EditorPage for TestPage {
    fn can_close(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn show(&self, ctx: &CtxRef) {
        Window::new("Settings").resizable(true).collapsible(true).show(ctx, |ui|{
            ScrollArea::auto_sized().show(ui, |ui|{
                ctx.settings_ui(ui);
            })
        });
    }
}

struct EditorAssets {
}

#[allow(dead_code)]
struct EditorInner {
    gui:    Egui,
    assets: Option<EditorAssets>,

    pages: Vec<Box<dyn EditorPage>>,
    selected_page: usize,
}

pub struct Editor(Mutex<EditorInner>);

impl Editor {
    fn lock(&self) -> MutexGuard<EditorInner> {
        self.0.lock().unwrap()
    }

    fn do_frame(&self, dt: f32) {
        let engine = Engine::as_ref();

        let graphics = engine.module::<Graphics>().unwrap();
        let device = graphics.device();

        let mut editor = self.lock();

        editor.gui.begin_frame(dt);

        let ctx = editor.gui.ctx().clone();

        TopPanel::top("title").show(&ctx, |ui|{
            menu::bar(ui, |ui|{
                let original_width = ui.available_width();

                let padding_y = 8.0;

                let mut style = (**ui.style()).clone();
                style.spacing.item_spacing = vec2(4.0, 0.0);
                style.spacing.button_padding = vec2(padding_y, padding_y);
                ui.set_style(style);

                let mut new_page = editor.selected_page;
                let mut remove_page = None;
                for (index, page) in editor.pages.iter().enumerate() {
                    let tab = MenuTab::new(index == editor.selected_page, page.name());

                    let response = ui.add(tab);

                    if response.clicked() {
                        new_page = index;
                    } else if response.middle_clicked() && page.can_close() {
                        remove_page = Some(index);
                        new_page = editor.selected_page - 1;
                    }
                }
                editor.selected_page = new_page;
                match remove_page {
                    Some(page) => {
                        editor.pages.remove(page);
                    },
                    None => { }
                }

                let mut style = (**ui.style()).clone();
                style.spacing.button_padding = vec2(padding_y + 5.0, padding_y);
                ui.set_style(style);

                // Take full width and fixed height:
                let height = ui.available_size().y;
                let size = vec2(ui.available_width() + 6.0, height);

                let used = original_width - size.x;

                ui.allocate_ui_with_layout(size, Layout::right_to_left(), |ui| {
                    let mut window = engine.window();
                    
                    let og_style = (**ui.style()).clone();

                    let mut style = (**ui.style()).clone();
                    style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(204, 36, 29);
                    style.visuals.widgets.active.bg_fill  = Color32::from_rgb(204, 36, 29);
                    ui.set_style(style);

                    if ui.button("🗙").clicked() {
                        engine.shutdown();
                    }

                    ui.set_style(og_style);

                    if ui.button("🗖").clicked() {
                        window.maximize();
                    }

                    if ui.button("🗕").clicked() {
                        window.minimize();
                    }

                    let right_used = size.x - ui.available_width();
                    
                    let space_left = ui.available_rect_before_wrap();
                    window.set_ignore_drag(!ui.rect_contains_pointer(space_left));

                    let title = Label::new(format!("{} - Newport Editor", engine.name()));
                    // TODO: Properly calculate the text width
                    if space_left.size().x >= 500.0 {
                        ui.add_space(used - right_used);
                    }

                    ui.centered_and_justified(|ui| {
                        ui.add(title);
                    });
                })
            })
        });

        let mut frame = Frame::default();
        frame.fill = Color32::from_rgb(52, 48, 47);

        if editor.selected_page >= editor.pages.len() {
            editor.selected_page = 0;
        }
        let page = &editor.pages[editor.selected_page];
        page.show(&ctx);

        let (_, clipped_meshes) = editor.gui.end_frame();
        device.update_bindless();

        let backbuffer = device.acquire_backbuffer();
        let mut gfx = device.create_graphics_context().unwrap();
        gfx.begin();
        {
            gfx.begin_render_pass(&graphics.backbuffer_render_pass(), &[&backbuffer]);
            gfx.clear((0.01, 0.01, 0.01, 1.0));
            editor.gui.draw(clipped_meshes, &mut gfx);
            gfx.end_render_pass();
        }
        gfx.resource_barrier_texture(&backbuffer, gpu::Layout::ColorAttachment, gpu::Layout::Present);
        gfx.end();
        
        let receipt = device.submit_graphics(vec![gfx], &[]);
        device.display(&[receipt]);
        device.wait_for_idle();
    }
}

impl Module for Editor {
    fn new() -> Self {
        let home = Box::new(HomePage);

        let mut pages: Vec<Box<dyn EditorPage>> = Vec::with_capacity(32);
        pages.push(home);

        for i in 0..5 {
            let page = Box::new(TestPage{
                name: format!("Test {}", i)
            });
            pages.push(page);
        }

        Self(Mutex::new(EditorInner{
            gui:    Egui::new(),
            assets: None,

            pages: pages,
            selected_page: 0,
        }))
    }

    fn depends_on(builder: EngineBuilder) -> EngineBuilder {
        builder
            .module::<Graphics>()
            .process_input(|engine: &Engine, window: &os::window::Window, event: &WindowEvent| {
                let mut editor = engine.module::<Editor>().unwrap().lock(); // SPEED: Maybe this will be too slow????
                editor.gui.process_input(window, event);
            })
            .tick(|engine: &Engine, dt: f32| {
                let editor = engine.module::<Editor>().unwrap();
                editor.do_frame(dt);
            })
    }
}