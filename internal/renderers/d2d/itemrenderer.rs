use i_slint_core::{
    api::PhysicalSize as PhysicalWindowSize,
    item_rendering::ItemRenderer,
    items::{
        BorderRectangle, BoxShadow, ClippedImage, ImageItem, ItemRc, Path, Rectangle, Text,
        TextInput,
    },
    lengths::{LogicalLength, LogicalPoint, LogicalRect, LogicalSize, LogicalVector},
    window::WindowInner,
    Brush, Color, RgbaColor,
};
use raw_window_handle::{HasRawWindowHandle, RawWindowHandle, Win32WindowHandle};
use std::{any::Any, pin::Pin};
use windows::{
    core::{ComInterface, Result},
    Win32::{
        Foundation::HWND,
        Graphics::{
            Direct2D::{
                Common::{
                    D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_PIXEL_FORMAT, D2D_RECT_F,
                    D2D_SIZE_U,
                },
                D2D1CreateFactory, ID2D1Brush, ID2D1Factory, ID2D1HwndRenderTarget,
                D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_FEATURE_LEVEL_DEFAULT,
                D2D1_HWND_RENDER_TARGET_PROPERTIES, D2D1_PRESENT_OPTIONS_NONE,
                D2D1_RENDER_TARGET_PROPERTIES, D2D1_RENDER_TARGET_TYPE_DEFAULT,
                D2D1_RENDER_TARGET_USAGE_NONE,
            },
            DirectWrite::{DWriteCreateFactory, IDWriteFactory, DWRITE_FACTORY_TYPE_SHARED},
            Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM,
        },
    },
};

pub struct D2DItemRenderer {
    d2d_factory: ID2D1Factory,
    dwrite_factory: IDWriteFactory,
    render_target: ID2D1HwndRenderTarget,
}

impl D2DItemRenderer {
    pub fn new(window: &impl HasRawWindowHandle) -> Result<Self> {
        unsafe {
            let d2d_factory: ID2D1Factory =
                D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
            let dwrite_factory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;

            let props = D2D1_RENDER_TARGET_PROPERTIES {
                r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                dpiX: 0.0,
                dpiY: 0.0,
                usage: D2D1_RENDER_TARGET_USAGE_NONE,
                minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
            };

            let hwnd = match window.raw_window_handle() {
                RawWindowHandle::Win32(Win32WindowHandle { hwnd, .. }) => hwnd,
                _ => panic!("D2D renderer is only supported with Win32WindowHandle"),
            };
            let hwnd_props = D2D1_HWND_RENDER_TARGET_PROPERTIES {
                hwnd: HWND(hwnd as _),
                pixelSize: D2D_SIZE_U::default(),
                presentOptions: D2D1_PRESENT_OPTIONS_NONE,
            };

            let render_target = d2d_factory.CreateHwndRenderTarget(&props, &hwnd_props)?;
            Ok(Self { d2d_factory, dwrite_factory, render_target })
        }
    }

    pub fn set_size(&self, size: PhysicalWindowSize, dpi: f32) -> Result<()> {
        unsafe {
            self.render_target.Resize(&D2D_SIZE_U { width: size.width, height: size.height })?;
            self.render_target.SetDpi(dpi, dpi);
        }
        Ok(())
    }

    pub fn begin_draw<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> Result<T> {
        unsafe {
            self.render_target.BeginDraw();
            let res = f(self);
            self.render_target.EndDraw(None, None)?;
            Ok(res)
        }
    }

    fn get_color(color: Color) -> D2D1_COLOR_F {
        let color = RgbaColor::<f32>::from(color);
        D2D1_COLOR_F { r: color.red, g: color.green, b: color.blue, a: color.alpha }
    }

    unsafe fn get_brush(&self, rect: &LogicalRect, brush: Brush) -> Result<ID2D1Brush> {
        let brush = match brush {
            Brush::SolidColor(c) => {
                self.render_target.CreateSolidColorBrush(&Self::get_color(c), None)?
            }
            Brush::LinearGradient(_) => todo!(),
            Brush::RadialGradient(_) => todo!(),
            _ => unreachable!(),
        };
        brush.cast()
    }
}

impl ItemRenderer for D2DItemRenderer {
    fn draw_rectangle(&mut self, rect: Pin<&Rectangle>, _self_rc: &ItemRc, _size: LogicalSize) {
        unsafe {
            let brush = self
                .get_brush(
                    &LogicalRect::new(
                        LogicalPoint::new(rect.x().0, rect.y().0),
                        LogicalSize::new(rect.width().0, rect.height().0),
                    ),
                    rect.background(),
                )
                .unwrap();
            let rect = D2D_RECT_F {
                left: rect.x().0,
                top: rect.y().0,
                right: rect.x().0 + rect.width().0,
                bottom: rect.y().0 + rect.height().0,
            };
            self.render_target.FillRectangle(&rect, &brush);
        }
    }

    fn draw_border_rectangle(
        &mut self,
        rect: Pin<&BorderRectangle>,
        _self_rc: &ItemRc,
        _size: LogicalSize,
    ) {
        todo!()
    }

    fn draw_image(&mut self, image: Pin<&ImageItem>, _self_rc: &ItemRc, _size: LogicalSize) {
        todo!()
    }

    fn draw_clipped_image(
        &mut self,
        image: Pin<&ClippedImage>,
        _self_rc: &ItemRc,
        _size: LogicalSize,
    ) {
        todo!()
    }

    fn draw_text(&mut self, text: Pin<&Text>, _self_rc: &ItemRc, _size: LogicalSize) {
        todo!()
    }

    fn draw_text_input(
        &mut self,
        text_input: Pin<&TextInput>,
        _self_rc: &ItemRc,
        _size: LogicalSize,
    ) {
        todo!()
    }

    fn draw_path(&mut self, path: Pin<&Path>, _self_rc: &ItemRc, _size: LogicalSize) {
        todo!()
    }

    fn draw_box_shadow(
        &mut self,
        box_shadow: Pin<&BoxShadow>,
        _self_rc: &ItemRc,
        _size: LogicalSize,
    ) {
        todo!()
    }

    fn combine_clip(
        &mut self,
        rect: LogicalRect,
        radius: LogicalLength,
        border_width: LogicalLength,
    ) -> bool {
        todo!()
    }

    fn get_current_clip(&self) -> LogicalRect {
        todo!()
    }

    fn translate(&mut self, distance: LogicalVector) {
        todo!()
    }

    fn rotate(&mut self, angle_in_degrees: f32) {
        todo!()
    }

    fn apply_opacity(&mut self, opacity: f32) {
        todo!()
    }

    fn save_state(&mut self) {
        todo!()
    }

    fn restore_state(&mut self) {
        todo!()
    }

    fn scale_factor(&self) -> f32 {
        todo!()
    }

    fn draw_cached_pixmap(
        &mut self,
        item_cache: &ItemRc,
        update_fn: &dyn Fn(&mut dyn FnMut(u32, u32, &[u8])),
    ) {
        todo!()
    }

    fn draw_string(&mut self, string: &str, color: Color) {
        todo!()
    }

    fn window(&self) -> &WindowInner {
        todo!()
    }

    fn as_any(&mut self) -> Option<&mut dyn Any> {
        todo!()
    }
}
