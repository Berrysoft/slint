use i_slint_backend_winit::WinitWindowAccessor;
use i_slint_core::{
    api::PhysicalSize as PhysicalWindowSize,
    graphics::FontRequest,
    item_rendering::ItemRenderer,
    items::TextInput,
    lengths::{LogicalLength, LogicalPoint, LogicalRect, LogicalSize, ScaleFactor},
    platform::PlatformError,
    renderer::Renderer,
    window::{WindowAdapter, WindowInner},
    Brush,
};
use std::{pin::Pin, rc::Weak};

mod itemrenderer;

pub struct D2DRenderer {
    window_adapter_weak: Weak<dyn WindowAdapter>,
}

impl D2DRenderer {
    pub fn new(window_adapter_weak: &Weak<dyn WindowAdapter>) -> Self {
        Self { window_adapter_weak: window_adapter_weak.clone() }
    }

    pub fn render(&self, size: PhysicalWindowSize) -> Result<(), PlatformError> {
        let window_adapter = self.window_adapter_weak.upgrade().unwrap();
        let window = WindowInner::from_pub(window_adapter.window());

        window.draw_contents(|components| -> Result<(), PlatformError> {
            let mut item_renderer = window_adapter
                .window()
                .with_winit_window(|window| itemrenderer::D2DItemRenderer::new(&window))
                .unwrap()
                .unwrap();
            item_renderer
                .begin_draw(|item_renderer| {
                    for (component, origin) in components {
                        i_slint_core::item_rendering::render_component_items(
                            component,
                            item_renderer,
                            *origin,
                        );
                    }
                })
                .unwrap();
            Ok(())
        })
    }
}

impl Renderer for D2DRenderer {
    fn text_size(
        &self,
        font_request: FontRequest,
        text: &str,
        max_width: Option<LogicalLength>,
        scale_factor: ScaleFactor,
    ) -> LogicalSize {
        todo!()
    }

    fn text_input_byte_offset_for_position(
        &self,
        text_input: Pin<&TextInput>,
        pos: LogicalPoint,
    ) -> usize {
        todo!()
    }

    fn text_input_cursor_rect_for_byte_offset(
        &self,
        text_input: Pin<&TextInput>,
        byte_offset: usize,
    ) -> LogicalRect {
        todo!()
    }

    fn default_font_size(&self) -> LogicalLength {
        todo!()
    }
}
