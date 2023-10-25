use image::{GenericImage, ImageBuffer, Rgb, RgbImage};
use log::debug;

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
        pub fn get_frame(
            frame_index: i32,
            image_buf_ptr: i32,
            image_buf_len: i32,
            image_buf_capacity: i32,
        ) -> i32;
        pub fn write_frame(frame_index: i32, image_buf_ptr: i32, image_buf_len: i32) -> i32;
    }
}

type FramesCount = i32;
type HostResultType = i32; // Can correspond 0 to okay, and num>0 to the equivalent of an error enum

fn process_video(mut filename: String) -> Result<(), ()> {
    let (mut width, mut height): (u32, u32) = (0, 0);
    let width_ptr = std::ptr::addr_of_mut!(width);
    let height_ptr = std::ptr::addr_of_mut!(height);

    let mut red_square = image::RgbImage::new(32, 32);
    for x in 0..32 {
        for y in 0..32 {
            red_square.put_pixel(x, y, Rgb([255, 0, 0]));
        }
    }

    let num_frames = unsafe {
        plugin::load_video(
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

    for idx in 0..num_frames {
        debug!("------ Run for frame {}", idx);
        let mut image_buf: Vec<u8> = vec![0; image_buf_size];

        let buf_ptr_raw = image_buf.as_mut_ptr() as usize as i32;
        let buf_len = image_buf.len() as i32;
        let buf_capacity = image_buf.capacity() as i32;
        debug!("WASM image_buf_ptr {:?}", buf_ptr_raw);
        debug!("WASM image_buf_len {:?}", buf_len);
        debug!("WASM image_buf_capacity {:?}", buf_capacity);
        // GET
        {
            unsafe { plugin::get_frame(idx, buf_ptr_raw, buf_len, buf_capacity) };
            let mut image_buf: ImageBuffer<image::Rgb<u8>, Vec<u8>> =
                ImageBuffer::from_vec(width, height, image_buf).unwrap();
            image_buf.copy_from(&red_square, 0, 0);
            // image_buf.save(format!("test{idx}.png"));
            unsafe { plugin::write_frame(idx, buf_ptr_raw, buf_len) };
        }

        let name = format!("wasm_save{idx}.png");
    }

    Ok(())
}

fn call_proc_vec() {
    let mut buf: Vec<u8> = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let buf_len = buf.len() as i32;
    let buf_capacity = buf.capacity() as i32;
    let buf_ptr_raw = buf.as_mut_ptr() as usize as i32;
    println!("WASM VEC {:?}", buf);
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
    // call_proc_vec();
    process_video("./times_square.mp4".to_string()).unwrap();
    // process_video("./video.mp4".to_string());

    Ok(())
}
