use std::{fs::File, io::prelude::*};

use ffmpeg::{
    encoder::Encoder,
    format::{self, input, Pixel},
    frame,
    media::Type,
    software::scaling::{context::Context, flag::Flags},
    util::frame::video::Video,
    Codec, Dictionary, Packet, Rational,
};

use crate::Frames;

#[derive(Copy, Clone)]
pub struct Width(pub u32);
#[derive(Copy, Clone)]
pub struct Height(pub u32);
#[derive(Copy, Clone)]
pub struct AspectRatio(pub Rational);
#[derive(Copy, Clone)]
pub struct FrameRate(pub Option<Rational>);

#[derive(Copy, Clone)]
pub struct VideoInfo {
    pub codec: Codec,
    pub format: Pixel,
    pub width: Width,
    pub height: Height,
    pub aspect_ratio: AspectRatio,
    pub frame_rate: FrameRate,
}

impl VideoInfo {
    pub fn new(
        codec: Codec,
        format: Pixel,
        width: Width,
        height: Height,
        aspect_ratio: AspectRatio,
        frame_rate: FrameRate,
    ) -> Self {
        VideoInfo {
            codec,
            format,
            width,
            height,
            aspect_ratio,
            frame_rate,
        }
    }

    pub fn width(&self) -> u32 {
        self.width.0
    }

    pub fn height(&self) -> u32 {
        self.height.0
    }
}

pub fn dump_frames(filename: &String) -> Result<(Frames, VideoInfo), ffmpeg::Error> {
    ffmpeg::init().unwrap();

    let mut frame_index = 0;
    let mut frames = Vec::new();
    let codec;
    let input = input(filename);
    let (width, height, aspect_ratio, frame_rate, format);

    match input {
        Ok(mut ictx) => {
            ictx.metadata();

            let input = ictx
                .streams()
                .best(Type::Video)
                .ok_or(ffmpeg::Error::StreamNotFound)?;
            let video_stream_index: usize = input.index();

            let mut decoder = input.decoder()?.video()?;
            // TODO: Proper Error handling
            codec = decoder.codec().unwrap();

            println!("CODEC {:?}", codec.name());
            println!("CODEC {:?}", codec.description());

            // I am wrapping these in Structs so its less likely that I make Type Errors
            width = Width(decoder.width());
            height = Height(decoder.height());
            aspect_ratio = AspectRatio(decoder.aspect_ratio());
            frame_rate = FrameRate(decoder.frame_rate());
            format = decoder.format();

            let mut scaler = Context::get(
                decoder.format(),
                decoder.width(),
                decoder.height(),
                Pixel::RGB24,
                decoder.width(),
                decoder.height(),
                Flags::BILINEAR,
            )?;

            let mut receive_and_process_decoded_frames =
                |decoder: &mut ffmpeg::decoder::Video| -> Result<(), ffmpeg::Error> {
                    let mut decoded = frame::Video::empty();
                    while decoder.receive_frame(&mut decoded).is_ok() {
                        let mut rgb_frame = Video::empty();
                        scaler.run(&decoded, &mut rgb_frame)?;
                        // save_file(&rgb_frame, frame_index).unwrap();
                        frames.push(rgb_frame);
                        frame_index += 1;
                    }
                    Ok(())
                };

            for res in ictx.packets() {
                let (stream, packet) = res?;
                if stream.index() == video_stream_index {
                    decoder.send_packet(&packet)?;
                    receive_and_process_decoded_frames(&mut decoder)?;
                }
            }
            decoder.send_eof()?;
            receive_and_process_decoded_frames(&mut decoder)?;
        }
        Err(err) => return Err(err),
    };

    let video_info = VideoInfo {
        codec,
        format,
        width,
        height,
        aspect_ratio,
        frame_rate,
    };

    Ok((frames, video_info))
}

// fn receive_and_process_encoded_packets(
//     // &mut self,
//     encoder:Encoder,
//     octx: &mut format::context::Output,
//     ost_time_base: Rational,
// ) {

//     let mut encoded = ffmpeg::codec::packet::packet::Packet::empty();

//     while encoder.receive_packet(&mut encoded).is_ok() {
//         //
//         encoded.set_stream(0);

//         encoded.rescale_ts(self.decoder.time_base().unwrap(), ost_time_base);
//         encoded.write_interleaved(octx).unwrap();
//     }
// }

// fn receive_and_process_encoded_packets(&mut self, octx: &mut format::context::Output, ost_time_base: Rational) {
//     let mut encoded = Packet::empty();
//     while self.encoder.receive_packet(&mut encoded).is_ok() {
//         encoded.set_stream(self.ost_index);
//         encoded.rescale_ts(self.decoder.time_base().unwrap(), ost_time_base);
//         encoded.write_interleaved(octx).unwrap();
//     }
// }

pub fn encode_frames(
    filename: &String,
    frames: &Vec<frame::Video>,
    v_info: VideoInfo,
) -> Result<Frames, ffmpeg::Error> {
    ffmpeg::init().unwrap();

    let mut octx = format::output(&filename).unwrap();

    let mut dict = Dictionary::new();
    dict.set("preset", "medium");

    let mut encoder = ffmpeg::codec::Encoder::new(v_info.codec)?.video()?;
    encoder.set_height(v_info.height.0);
    encoder.set_width(v_info.width.0);
    encoder.set_aspect_ratio(v_info.aspect_ratio.0);
    encoder.set_format(v_info.format);
    encoder.set_frame_rate(v_info.frame_rate.0);
    encoder.set_time_base(Some(v_info.frame_rate.0.unwrap().invert()));

    // TODO: Do i need this ?
    // if global_header {
    //     encoder.set_flags(codec::Flags::GLOBAL_HEADER);
    // }

    let encoder: ffmpeg::encoder::Video = encoder
        .open_with(dict)
        .expect("error opening libx264 encoder with supplied settings");

    for frame in frames {
        // let mut packet = frame.packet();
        // encoder.receive_packet(&mut packet);
        // encoder.send_frame(frame).unwrap();
    }

    // encoder.
    // TODO:Continue from here
    // ffmpeg::encoder::Video::

    todo!();
}

fn receive_and_process_decoded_frames(
    frames: Vec<frame::Video>,
    octx: &mut format::context::Output,
    ost_time_base: Rational,
) {
    let mut frame_count = 0;
    for (idx, mut frame) in frames.into_iter().enumerate() {
        println!("Frame {idx}");
        frame_count += 1;
        // let timestamp = frame.timestamp();
        let timestamp: Option<i64> = Some((idx * 1000) as i64);
        // frame.set_pts(timestamp);
        // frame.set_kind(ffmpeg::picture::Type::None);
        // self.send_frame_to_encoder(&frame);
        // self.receive_and_process_encoded_packets(octx, ost_time_base);
    }
}

fn receive_and_process_encoded_packets(
    encoder: &mut ffmpeg::encoder::Video,
    ost_index: usize,
    octx: &mut format::context::Output,
    ost_time_base: Rational,
    decoder_time_base: Rational,
) {
    let mut encoded = Packet::empty();
    while encoder.receive_packet(&mut encoded).is_ok() {
        encoded.set_stream(ost_index);
        encoded.rescale_ts(decoder_time_base, ost_time_base);
        let write_interleaved = encoded.write_interleaved(octx);
        if let Err(err) = write_interleaved {
            println!("write_interleaved {:?}", err);
        };
    }
}

pub fn save_file(frame: &Video, index: usize) -> std::result::Result<(), std::io::Error> {
    let mut file = File::create(format!("frame{}.ppm", index))?;
    file.write_all(format!("P6\n{} {}\n255\n", frame.width(), frame.height()).as_bytes())?;
    file.write_all(frame.data(0))?;
    Ok(())
}
