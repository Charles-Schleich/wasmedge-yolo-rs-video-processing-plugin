use ffmpeg::{
    codec, dictionary, encoder,
    format::{input, Pixel},
    frame,
    media::Type,
    software::scaling::{context::Context, flag::Flags},
    util::frame::video::Video,
    Rational,
};

use crate::{AspectRatio, FrameRate, Frames, Height, VideoInfo, Width};

pub fn dump_frames(filename: &String) -> Result<(Frames, VideoInfo), ffmpeg::Error> {
    ffmpeg::init().unwrap();

    let mut frame_index = 0;
    let mut frames = Vec::new();
    let codec;
    let input = input(filename);
    let (width, height, aspect_ratio, frame_rate, format);
    let input_stream_meta_data: dictionary::Owned;
    let ost_time_bases;
    let itcx_number_streams;
    let decoder_time_base;

    match input {
        Ok(mut ictx) => {
            let input = ictx
                .streams()
                .best(Type::Video)
                .ok_or(ffmpeg::Error::StreamNotFound)?;
            itcx_number_streams = ictx.nb_streams();

            ost_time_bases = vec![Rational(0, 0); ictx.nb_streams() as _];

            let video_stream_index: usize = input.index();

            input_stream_meta_data = ictx.metadata().to_owned();

            let mut decoder = input.decoder()?.video()?;
            // TODO no Unwrap
            decoder_time_base = decoder.time_base().unwrap();

            // TODO: Proper Error handling
            // codec = decoder.codec().unwrap();
            codec = encoder::find(codec::Id::H264).unwrap();

            println!("Decoder Codec {:?}", codec.name());
            println!("Decoder Codec {:?}", codec.description());

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
        input_stream_meta_data,
        itcx_number_streams,
        decoder_time_base,
    };

    Ok((frames, video_info))
}
