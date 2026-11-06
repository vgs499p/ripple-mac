use resvg::{tiny_skia, usvg};
fn main() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    for (input, output, width) in [
        ("site/ripple.svg", "ui/logo.png", 240.0),
        ("site/masthead.svg", "ui/header.png", 1600.0),
    ] {
        let tree = usvg::Tree::from_data(
            &std::fs::read(root.join(input)).unwrap(),
            &usvg::Options::default(),
        )
        .unwrap();
        let scale = width / tree.size().width();
        let mut pixels =
            tiny_skia::Pixmap::new(width as u32, (tree.size().height() * scale).ceil() as u32)
                .unwrap();
        resvg::render(
            &tree,
            tiny_skia::Transform::from_scale(scale, scale),
            &mut pixels.as_mut(),
        );
        pixels.save_png(root.join(output)).unwrap();
    }
}
