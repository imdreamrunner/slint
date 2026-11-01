// Copyright © SixtyFPS GmbH <info@slint.dev>
// SPDX-License-Identifier: GPL-3.0-only OR LicenseRef-Slint-Royalty-free-2.0 OR LicenseRef-Slint-Software-3.0

use i_slint_core::graphics::euclid;
use i_slint_core::lengths::LogicalRect;
use i_slint_core::renderer::RendererSealed;
use slint::platform::software_renderer::{
    LineBufferProvider, MinimalSoftwareWindow, PhysicalRegion, RenderingRotation, Rgb565Pixel,
    TargetPixel,
};
use slint::{ComponentHandle, Image, Rgb8Pixel, SharedPixelBuffer};

slint::slint! {
    #[edition = "1.19"]
    export component RoundedImageScene inherits Window {
        width: 96px;
        height: 80px;
        background: #202830;
        in property <image> picture;
        in property <bool> nested;
        in property <length> stroke;

        Rectangle {
            x: 8px;
            y: 8px;
            width: 64px;
            height: 64px;
            border-radius: 20px;
            border-width: root.stroke;
            border-color: #6080ff;
            clip: true;
            Rectangle {
                x: root.nested ? 8px : 0px;
                y: root.nested ? 4px : 0px;
                width: root.nested ? 48px : 64px;
                height: root.nested ? 56px : 64px;
                border-radius: root.nested ? 16px : 0px;
                clip: root.nested;
                Image {
                    x: -8px;
                    y: -8px;
                    width: 80px;
                    height: 80px;
                    source: root.picture;
                    image-fit: fill;
                }
            }
        }
        Rectangle {
            x: 80px;
            y: 16px;
            width: 8px;
            height: 48px;
            background: #ffc040;
        }
    }
}

struct Lines<'a, T> {
    pixels: &'a mut [T],
    stride: usize,
}

impl<T: TargetPixel> LineBufferProvider for Lines<'_, T> {
    type TargetPixel = T;

    fn process_line(
        &mut self,
        line: usize,
        range: core::ops::Range<usize>,
        render: impl FnOnce(&mut [T]),
    ) {
        let start = line * self.stride + range.start;
        render(&mut self.pixels[start..][..range.len()]);
    }
}

fn render<T: TargetPixel>(
    window: &MinimalSoftwareWindow,
    rotation: RenderingRotation,
    by_line: bool,
    dirty: Option<LogicalRect>,
    pixels: &mut [T],
    stride: usize,
) -> PhysicalRegion {
    window.request_redraw();
    let mut result = None;
    assert!(window.draw_if_needed(|renderer| {
        renderer.set_rendering_rotation(rotation);
        if let Some(dirty) = dirty {
            renderer.mark_dirty_region(dirty.into());
        }
        result = Some(if by_line {
            renderer.render_by_line(Lines { pixels, stride })
        } else {
            renderer.render(pixels, stride)
        });
    }));
    result.unwrap()
}

fn rotated_point(
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    rotation: RenderingRotation,
) -> usize {
    match rotation {
        RenderingRotation::NoRotation => y * width + x,
        RenderingRotation::Rotate90 => x * height + height - y - 1,
        RenderingRotation::Rotate180 => (height - y - 1) * width + width - x - 1,
        RenderingRotation::Rotate270 => (width - x - 1) * height + y,
        _ => unreachable!(),
    }
}

fn check_partial_updates<T: TargetPixel + core::fmt::Debug + PartialEq>() {
    let window = crate::software::init_swr();
    let scene = RoundedImageScene::new().unwrap();
    let solid_image = |r, g, b| {
        let mut pixels = SharedPixelBuffer::new(2, 2);
        pixels.make_mut_slice().fill(Rgb8Pixel::new(r, g, b));
        Image::from_rgb8(pixels)
    };
    scene.show().unwrap();
    let full = LogicalRect::new(euclid::point2(0., 0.), euclid::size2(96., 80.));

    for scale in [1., 1.5] {
        window.dispatch_event(slint::platform::WindowEvent::ScaleFactorChanged {
            scale_factor: scale,
        });
        let width = (96. * scale) as usize;
        let height = (80. * scale) as usize;
        window.set_size(slint::PhysicalSize::new(width as u32, height as u32));
        for rotation in [
            RenderingRotation::NoRotation,
            RenderingRotation::Rotate90,
            RenderingRotation::Rotate180,
            RenderingRotation::Rotate270,
        ] {
            let stride = match rotation {
                RenderingRotation::Rotate90 | RenderingRotation::Rotate270 => height,
                _ => width,
            };
            for nested in [false, true] {
                scene.set_nested(nested);
                for stroke in [0., 3.] {
                    scene.set_stroke(stroke);
                    for by_line in [false, true] {
                        scene.set_picture(solid_image(0, 255, 0));
                        let mut pixels = vec![T::from_rgb(255, 0, 255); width * height];
                        render(&window, rotation, by_line, Some(full), &mut pixels, stride);
                        let corner = rotated_point(
                            (8. * scale) as usize,
                            (8. * scale) as usize,
                            width,
                            height,
                            rotation,
                        );
                        assert_eq!(pixels[corner], T::from_rgb(0x20, 0x28, 0x30));

                        scene.set_picture(solid_image(255, 0, 0));
                        let dirty = render(&window, rotation, by_line, None, &mut pixels, stride);
                        let area: u32 =
                            dirty.iter().map(|(_, size)| size.width * size.height).sum();
                        assert!(area > 0 && area < (width * height) as u32, "{dirty:?}");
                        let mut expected = vec![T::from_rgb(255, 0, 255); width * height];
                        render(&window, rotation, by_line, Some(full), &mut expected, stride);
                        assert_eq!(
                            pixels, expected,
                            "{rotation:?}, scale={scale}, nested={nested}, stroke={stroke}, by_line={by_line}"
                        );
                        for (x, y, color) in [
                            (40., 40., T::from_rgb(255, 0, 0)),
                            (84., 40., T::from_rgb(255, 192, 64)),
                        ] {
                            let index = rotated_point(
                                (x * scale) as usize,
                                (y * scale) as usize,
                                width,
                                height,
                                rotation,
                            );
                            assert_eq!(pixels[index], color);
                        }

                        // Repaint only the corners, including pixels outside the rounded clip.
                        for (x, y) in [(8, 8), (48, 48)] {
                            let (px, py, side) = (
                                (x as f32 * scale) as usize,
                                (y as f32 * scale) as usize,
                                (24. * scale) as usize,
                            );
                            for y in py..py + side {
                                for x in px..px + side {
                                    pixels[rotated_point(x, y, width, height, rotation)] =
                                        T::from_rgb(255, 0, 255);
                                }
                            }
                            let region = LogicalRect::new(
                                euclid::point2(x as f32, y as f32),
                                euclid::size2(24., 24.),
                            );
                            render(&window, rotation, by_line, Some(region), &mut pixels, stride);
                            assert_eq!(
                                pixels, expected,
                                "corner repaint: {rotation:?}, scale={scale}, nested={nested}, stroke={stroke}, by_line={by_line}"
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn rounded_clipping_partial_updates_rgb888() {
    check_partial_updates::<Rgb8Pixel>();
}

#[test]
fn rounded_clipping_partial_updates_rgb565() {
    check_partial_updates::<Rgb565Pixel>();
}
