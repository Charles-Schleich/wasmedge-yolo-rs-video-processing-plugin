use std::ops::Range;

mod plugin {
    use crate::{FramesCount, HostResultType};

    #[link(wasm_import_module = "yolo-video-proc")]
    extern "C" {
        // Example functions
        pub fn proc_vec(ext_ptr: i32, buf_len: i32, capacity: i32) -> i32;
        pub fn proc_string(ext_ptr: i32, buf_len: i32, capacity: i32) -> i32;
        //
        pub fn load_video(
            str_ptr: i32,
            str_len: i32,
            str_capacity: i32,
            width_ptr: *mut u32,
            height_ptr: *mut u32,
        ) -> FramesCount;
        pub fn get_image_meta_data(width: i32, height: i32, bytes_length: i32) -> HostResultType;
        pub fn get_frame(frame_index: i32, frame_ptr: i32, frame_len: i32) -> i32;
    }
}

type FramesCount = i32;
type HostResultType = i32; // Can correspond 0 to okay, and num>0 to the equivalent of an error enum

fn process_video(mut filename: String) -> Vec<image::DynamicImage> {
    let (mut width, mut height): (u32, u32) = (0, 0);
    let width_ptr = std::ptr::addr_of_mut!(width);
    let height_ptr = std::ptr::addr_of_mut!(height);

    let num_frames = unsafe {
        plugin::load_video(
            filename.as_mut_ptr() as usize as i32,
            filename.len() as i32,
            filename.capacity() as i32,
            width_ptr,
            height_ptr,
        )
    };

    println!("WIDTH {}", width);
    println!("HEIGHT {}", height);

    println!("Woop woop {}", num_frames);
    let mut frame = "10".to_string();

    // for idx in 0..num_frames {
    for idx in [0, 10, 20, 30, 40] {
        println!("------ Run for frame {}", idx);
        let frame_length = 100;
        unsafe { plugin::get_frame(idx, frame.as_mut_ptr() as usize as i32, frame_length) };
        todo!("Change Frame length");
        println!("------ Run for frame {}", idx);
    }

    // let mut images = Vec::<image::DynamicImage>::with_capacity(num_frames as usize);
    todo!();
}

fn call_proc_vec() {
    let mut buf: Vec<u8> = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let buf_len = buf.len() as i32;
    let buf_capacity = buf.capacity() as i32;
    let buf_ptr_raw = buf.as_mut_ptr() as usize as i32;
    println!("Before Function Call '{:?}'", buf);
    let y = unsafe { plugin::proc_vec(buf_ptr_raw, buf_len, buf_capacity) };
    println!("After Function Call '{:?}'", buf);
}

fn call_proc_string() {
    let mut s = "hello plugin".to_string();
    println!("Before Function Call '{}'", s);
    let y = unsafe {
        plugin::proc_string(
            s.as_mut_ptr() as usize as i32,
            s.len() as i32,
            s.capacity() as i32,
        )
    };
    println!("After Function Call {}", s);
    println!("Function Output {}", y);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // process_video("./times_square.mp4".to_string());
    // process_video("./dog.mp4".to_string());
    process_video("./video.mp4".to_string());
    // process_video("./demo_video/out.mp4".to_string());
    // process_video("./demo_video/out_YUV.mp4".to_string());
    // process_video("./green_vid.mp4".to_string());
    // process_video("./RED_vid.mp4".to_string());
    // process_video("./video_fixed.mp4".to_string());
    // process_video("./small_Red.mp4".to_string());

    Ok(())
}
