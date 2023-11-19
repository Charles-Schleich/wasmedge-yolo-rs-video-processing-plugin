#[macro_use]
extern crate log;
extern crate simplelog;

use image::{GenericImage, ImageBuffer, Rgb};
use log::{debug, info, LevelFilter};
use prgrs::Prgrs;
use simplelog::{ColorChoice, CombinedLogger, Config, TermLogger, TerminalMode};

mod plugin {
    use log::LevelFilter;

    pub fn init_plugin_logging_with_log_level(level_filter: LevelFilter) {
        let level_filter_i32 = level_filter as i32;
        let level_filter_ptr = std::ptr::addr_of!(level_filter_i32);
        unsafe { init_plugin_logging(level_filter_ptr) };
    }

    #[link(wasm_import_module = "yolo-video-proc")]
    extern "C" {
        pub fn init_plugin_logging(level: *const i32) -> i32;

        pub fn load_video_to_host_memory(
            str_ptr: i32,
            str_len: i32,
            str_capacity: i32,
            width_ptr: *mut i32,
            height_ptr: *mut i32,
            frame_count: *mut i32,
        ) -> i32;

        pub fn get_frame(
            frame_index: i32,
            image_buf_ptr: i32,
            image_buf_len: i32,
            image_buf_capacity: i32,
        ) -> i32;

        pub fn write_frame(frame_index: i32, image_buf_ptr: i32, image_buf_len: i32) -> i32;

        pub fn assemble_output_frames_to_video(
            str_ptr: i32,
            str_len: i32,
            str_capacity: i32,
        ) -> i32;

    }
}

fn process_video(mut filename: String, fail: u32) -> Result<(), ()> {
    debug!("Start Proc Video");

    plugin::init_plugin_logging_with_log_level(LevelFilter::Info);

    let (mut width, mut height, mut frame_count): (i32, i32, i32) = (0, 0, 10);
    let width_ptr = std::ptr::addr_of_mut!(width);
    let height_ptr = std::ptr::addr_of_mut!(height);
    let frame_count_ptr = std::ptr::addr_of_mut!(frame_count);

    let mut red_square = image::RgbImage::new(32, 32);
    let mut blue_square = image::RgbImage::new(32, 32);
    for x in 0..32 {
        for y in 0..32 {
            red_square.put_pixel(x, y, Rgb([255, 0, 0]));
            blue_square.put_pixel(x, y, Rgb([0, 0, 255]));
        }
    }

    debug!("Call load_video_to_host_memory() ");
    let result = unsafe {
        plugin::load_video_to_host_memory(
            filename.as_mut_ptr() as usize as i32,
            filename.len() as i32,
            filename.capacity() as i32,
            width_ptr,
            height_ptr,
            frame_count_ptr,
        )
    };

    if fail == 1 {
        return Ok(());
    }

    let image_buf_size: usize = (width * height * 3) as usize;
    debug!("WIDTH {}", width);
    debug!("HEIGHT {}", height);
    debug!("Number of Frames {}", frame_count);

    info!("Begin Processing {} frames ", frame_count);

    for idx in Prgrs::new(0..frame_count, frame_count as usize) {
        debug!("------ Run for frame {}", idx);
        let mut image_buf: Vec<u8> = vec![0; image_buf_size];

        let buf_ptr_raw = image_buf.as_mut_ptr() as usize as i32;
        let buf_len = image_buf.len() as i32;
        let buf_capacity = image_buf.capacity() as i32;
        debug!("WASM image_buf_ptr {:?}", buf_ptr_raw);
        debug!("WASM image_buf_len {:?}", buf_len);
        debug!("WASM image_buf_capacity {:?}", buf_capacity);

        {
            unsafe { plugin::get_frame(idx, buf_ptr_raw, buf_len, buf_capacity) };
            let mut image_buf: ImageBuffer<image::Rgb<u8>, Vec<u8>> =
                ImageBuffer::from_vec(width as u32, height as u32, image_buf).unwrap();
            let _ = image_buf.copy_from(&red_square, 0, 0);
            let _ = image_buf.copy_from(&blue_square, 64, 64);

            unsafe { plugin::write_frame(idx, buf_ptr_raw, buf_len) };
        }
    }

    info!("Finished Writing {:?} Frames To Plugin", frame_count);

    let mut out: Vec<&str> = filename.split(".").collect::<Vec<&str>>();
    out.insert(0, "./");
    out.insert(out.len() - 1, "_out.");
    let mut output_filename = out.join("");

    info!("Begin Encode Video {:?}", output_filename);
    let output_code = unsafe {
        plugin::assemble_output_frames_to_video(
            output_filename.as_mut_ptr() as usize as i32,
            output_filename.len() as i32,
            output_filename.capacity() as i32,
        )
    };

    info!("Finished Encoding Video : {}", output_filename);

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    CombinedLogger::init(vec![TermLogger::new(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Always,
    )])
    .unwrap();

    // process_video("./1080p60.mp4".to_string(), 1).unwrap();
    // process_video("./ts_wide.mp4".to_string()).unwrap();
    // process_video("./times_square.mp4".to_string()).unwrap();
    process_video("small_bunny_1080p_60fps.mp4".to_string(), 0).unwrap();

    Ok(())
}
