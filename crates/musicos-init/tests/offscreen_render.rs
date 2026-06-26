use musicos_init::app::MusicOs;
use musicos_init::renderer::{Canvas, Resolution};

#[test]
fn renders_setup_screen_at_both_supported_resolutions() {
    for resolution in [Resolution::HD_4_3, Resolution::VGA_4_3] {
        let mut buffer = vec![0; resolution.width as usize * resolution.height as usize * 4];
        let mut canvas = Canvas::new(resolution, &mut buffer).expect("canvas");
        let mut app = MusicOs::new(resolution);
        app.draw(&mut canvas, 0.75);
        assert!(
            buffer.iter().any(|byte| *byte != 0),
            "rendered buffer should not be blank for {:?}",
            resolution
        );
    }
}
