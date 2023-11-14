use image::{GenericImage, ImageBuffer, Rgb};
use log::{debug, info};

mod plugin {
    use crate::{FramesCount, HostResultType};

    #[link(wasm_import_module = "yolo-video-proc")]
    extern "C" {
        // Example functions

        pub fn load_video_to_host_memory(
            str_ptr: i32,
            str_len: i32,
            str_capacity: i32,
            width_ptr: *mut u32,
            height_ptr: *mut u32,
        ) -> FramesCount;

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
        ) -> HostResultType;

    }
}

type FramesCount = i32;
type HostResultType = i32; // Can correspond 0 to okay, and num>0 to the equivalent of an error enum

fn process_video(mut filename: String) -> Result<(), ()> {
    debug!("Start Proc Video");

    let (mut width, mut height): (u32, u32) = (0, 0);
    let width_ptr = std::ptr::addr_of_mut!(width);
    let height_ptr = std::ptr::addr_of_mut!(height);

    let mut red_square = image::RgbImage::new(32, 32);
    let mut blue_square = image::RgbImage::new(32, 32);
    for x in 0..32 {
        for y in 0..32 {
            red_square.put_pixel(x, y, Rgb([255, 0, 0]));
            blue_square.put_pixel(x, y, Rgb([0, 0, 255]));
            // red_square.put_pixel(x, y, Rgb([255, 0, 0]));
        }
    }

    let num_frames = unsafe {
        plugin::load_video_to_host_memory(
            filename.as_mut_ptr() as usize as i32,
            filename.len() as i32,
            filename.capacity() as i32,
            width_ptr,
            height_ptr,
        )
    };

    let image_buf_size: usize = (width * height * 3) as usize;
    debug!("WIDTH {}", width);
    debug!("HEIGHT {}", height);
    debug!("Number of Frames {}", num_frames);

    info!("# frames to Process: {}", num_frames);
    for idx in 0..num_frames {
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
                ImageBuffer::from_vec(width, height, image_buf).unwrap();
            let _ = image_buf.copy_from(&red_square, 0, 0);
            let _ = image_buf.copy_from(&blue_square, 64, 64);

            unsafe { plugin::write_frame(idx, buf_ptr_raw, buf_len) };
        }
    }
    info!("Finished Writing Frames {:?}", num_frames);

    // let mut output_filename: String = format!("video_output.mp4");
    let mut out: Vec<&str> = filename.split(".").collect::<Vec<&str>>();
    out.insert(0, "./");
    out.insert(out.len() - 1, "_out.");
    let mut output_filename = out.join("");
    debug!("Output Filename {}", output_filename);
    let output_code = unsafe {
        plugin::assemble_output_frames_to_video(
            output_filename.as_mut_ptr() as usize as i32,
            output_filename.len() as i32,
            output_filename.capacity() as i32,
        )
    };

    debug!("Out Code: {}", output_code);
    debug!("Finished: {}", output_filename);

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // process_video("./1080p60.mp4".to_string()).unwrap();
    // process_video("./ts_wide.mp4".to_string()).unwrap();
    // process_video("./times_square.mp4".to_string()).unwrap();
    process_video("small_bunny_1080p_60fps.mp4".to_string()).unwrap();

    Ok(())
}
