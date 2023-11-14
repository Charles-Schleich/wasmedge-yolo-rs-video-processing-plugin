use std::{fs::File, io::prelude::*, time::Duration};

use ffmpeg::{
    codec,
    ffi::EAGAIN,
    format::{self, Pixel},
    frame, option, picture,
    software::scaling::{Context, Flags},
    util::frame::video::Video,
    Dictionary, Packet, Rational, Rescale,
};

use std::collections::BTreeMap;

use ffmpeg::Error as AvError;

use crate::{time::Time, VideoInfo};

enum EncoderError {
    FrameMapIsIncomplete(Vec<usize>), // TODO FILL THIS
}

pub(crate) struct VideoEncoder {
    // Encoder
    encoder: ffmpeg::encoder::Video,
    // Output Context
    octx: ffmpeg::format::context::output::Output,
    // Output Time Base
    packet_order_map: BTreeMap<i64, Packet>, // ost_time_bases: Vec<Rational>,
}

impl VideoEncoder {
    pub fn new(v_info: VideoInfo, output_file: &String) -> Self {
        let mut octx = format::output(&output_file).unwrap();

        let global_header = octx.format().flags().contains(format::Flags::GLOBAL_HEADER);
        let mut ost = octx.add_stream().unwrap();

        let codec = ffmpeg::encoder::find(codec::Id::H264).unwrap();
        // libx264rgb

        println!("ENCODER CODEC : {:?}", codec.name());

        let mut encoder = ffmpeg::codec::Encoder::new(codec).unwrap().video().unwrap();

        // println!("=======\nINIT encoder");
        // println!("Settings {:#?}", v_info);
        let frame_rate = v_info.frame_rate.0;
        // let mut ost_time_bases = vec![Rational(0, 0); v_info.itcx_number_streams as _];

        encoder.set_height(v_info.height.0);
        encoder.set_width(v_info.width.0);
        encoder.set_format(v_info.format);
        encoder.set_time_base(Some(ffmpeg::rescale::TIME_BASE));
        encoder.set_frame_rate(Some((60, 1)));
        println!("encoder.time_base {:?}", encoder.time_base());

        // NEXT THING TO TRY:
        //     Manually KEEP TRACK OF AND ADD
        //     let duration: Time = Duration::from_nanos(1_000_000_000 / 24).into();
        //     let mut position = Time::zero();
        //     FROM EXAMPLE TO THE RIGHT
        //     ALSO WRITE FLUSH FUNCTION

        // encoder.set_bit_rate(v_info.bitrate.0);
        // encoder.set_max_bit_rate(v_info.max_bitrate.0);
        // println!("END INIT encoder\n=======");

        let mut dict = Dictionary::new();
        // dict.set("preset", "slow");

        let mut encoder: ffmpeg::encoder::Video = encoder
            .open_with(dict)
            .expect("error opening libx264 encoder with supplied settings");

        // println!("==================");
        // println!("Encoder Parameters");
        let enc_params = encoder.parameters();
        // println!("id : {:?}", enc_params.id());
        // println!("tag: {:?}", enc_params.tag());
        // println!("medium {:?}", enc_params.medium());
        // println!("==================");

        ost.set_parameters(encoder.parameters());
        // let ost_time_base = ost_time_bases[ost_index as usize];

        if global_header {
            // println!(
            //     "Setting Global header Flag : {:?}",
            //     codec::Flags::GLOBAL_HEADER
            // );
            encoder.set_flags(codec::Flags::GLOBAL_HEADER);
        }

        // println!("==========================");
        // println!("Write output context");
        octx.set_metadata(v_info.input_stream_meta_data);
        format::context::output::dump(&octx, 0, Some(&output_file));
        octx.write_header().unwrap();
        // println!("==========================");

        VideoEncoder {
            encoder,
            octx,
            packet_order_map: BTreeMap::new(), // ost_time_bases,
                                               // decoder_time_base: v_info.decoder_time_base,
        }
    }

    pub fn receive_and_process_decoded_frames(
        &mut self,
        frames: Vec<(&mut frame::Video, picture::Type, Option<i64>)>,
    ) -> Result<(), ()> {
        println!("frames.len() {}", frames.len());
        // let mut frame_count = 0;

        // Write Every Frame out to encoder packet
        let mut scaler = Context::get(
            Pixel::RGB24,
            self.encoder.width(),
            self.encoder.height(),
            Pixel::YUV420P,
            self.encoder.width(),
            self.encoder.height(),
            Flags::empty(),
            // Flags::BILINEAR,
        )
        .unwrap();
        let duration: Time = Duration::from_nanos(1_000_000_000 / 60).into();
        println!("duration {}", duration);
        let mut position = Time::zero();

        for (idx, (out_frame_rgb, frame_type, frame_timestamp)) in frames.into_iter().enumerate() {
            // println!(" FR {idx} {:?} , ", frame_timestamp);
            let mut frame_yuv420 = Video::empty();

            let frame_timestamp_rescale = position
                .aligned_with_rational(self.encoder.time_base().unwrap())
                .into_value();

            scaler.run(&out_frame_rgb, &mut frame_yuv420).unwrap();
            frame_yuv420.set_pts(frame_timestamp_rescale);

            if frame_type == picture::Type::I {
                frame_yuv420.set_kind(picture::Type::I);
            } else {
                frame_yuv420.set_kind(picture::Type::None)
            }

            println!("F Send {:?} {}",frame_yuv420.pts(),frame_yuv420.display_number());
            self.encoder.send_frame(&frame_yuv420).unwrap();

            if let Some(packet) = self.encoder_receive_packet()? {
                self.packet_order_map.insert(packet.pts().unwrap(), packet);
                // self.write_encoded_packets(&mut packet,0);
            }

            // Increase position
            let aligned_position = position.aligned_with(&duration);
            // println!("  Pos Time: {:?}", position);
            position = aligned_position.add();
        }

        // self.write_encoded_packets(&mut packet,0);

        while let Some((k, mut packet)) = self.packet_order_map.pop_first() {
            println!("Writing Packet {:?}", k);
            self.write_encoded_packets(&mut packet, 0);
        }

        // REPLACE ALL THE BELOW WITH FLUSH  FUNCTION
        self.finish();
        // Send End of file information to Encoder and Output Context
        // self.encoder.send_eof().unwrap();
        // self.receive_and_process_encoded_packets(0, None);
        // if let Err(err) = self.octx.flush() {
        //     println!("Error: {}", err);
        // };
        // self.octx.write_trailer().unwrap();

        return Ok(());
    }

    fn flush(&mut self) -> Result<(), ()> {
        // Maximum number of invocations to `encoder_receive_packet`
        // to drain the items still on the queue before giving up.
        const MAX_DRAIN_ITERATIONS: u32 = 100;

        // Notify the encoder that the last frame has been sent.
        self.encoder.send_eof().unwrap();

        // We need to drain the items still in the encoders queue.
        for _ in 0..MAX_DRAIN_ITERATIONS {
            let mut packet = Packet::empty();
            match self.encoder.receive_packet(&mut packet) {
                Ok(_) => self.write_encoded_packets(&mut packet, 0),
                Err(_) => break,
            };
        }

        Ok(())
    }

    pub fn finish(&mut self) -> Result<(), ()> {
        self.flush()?;
        self.octx.write_trailer().unwrap();
        Ok(())
    }

    fn encoder_receive_packet(&mut self) -> Result<Option<Packet>, ()> {
        let mut packet = Packet::empty();
        let encode_result = self.encoder.receive_packet(&mut packet);
        match encode_result {
            Ok(()) => Ok(Some(packet)),
            Err(AvError::Io(errno)) => {
                println!("IO error {}", errno);
                Ok(None)
            }
            Err(err) => Err(()),
        }
    }

    fn write_encoded_packets(&mut self, packet: &mut Packet, ost_index: usize) {
        // let mut encoded_packet = Packet::empty();
        packet.set_stream(ost_index);
        packet.set_position(-1);
        println!("P Write S {:?} {:?} {:?}",packet.pts(), packet.dts(), packet.duration());

        packet.rescale_ts(
            self.encoder.time_base().unwrap(),
            self.octx.stream(0).unwrap().time_base().unwrap(),
        );
        packet.set_dts(packet.pts().map(|c| c - 3000));
        println!(
            "P Write F {:?} {:?}",
            packet.pts(),
            packet.dts()
        );
        // println!(
        //     "rescale TS  SRC: {} - DST:{}",
        //     self.encoder.time_base().unwrap(),
        //     self.octx.stream(0).unwrap().time_base().unwrap()
        // );

        // let write_interleaved = encoded_packet.write(&mut self.octx);
        let write_interleaved = packet.write_interleaved(&mut self.octx);
        if let Err(err) = write_interleaved {
            println!("write_interleaved {:?}", err);
        };
    }

    // fn receive_and_process_encoded_packets(&mut self, ost_index: usize) {
    //     let mut encoded_packet = Packet::empty();
    //     while self.encoder.receive_packet(&mut encoded_packet).is_ok() {
    //         encoded_packet.set_stream(ost_index);
    //         encoded_packet.set_position(-1);
    //         println!("P Write S {:?}", encoded_packet.pts());
    //         encoded_packet.rescale_ts(
    //             self.encoder.time_base().unwrap(),
    //             self.octx.stream(0).unwrap().time_base().unwrap(),
    //         );
    //         println!("P Write F {:?}", encoded_packet.pts());
    //         println!(
    //             "rescale TS  SRC: {} - DST:{}",
    //             self.encoder.time_base().unwrap(),
    //             self.octx.stream(0).unwrap().time_base().unwrap()
    //         );

    //         let write_interleaved = encoded_packet.write(&mut self.octx);
    //         // let write_interleaved = encoded_packet.write_interleaved(&mut self.octx);
    //         if let Err(err) = write_interleaved {
    //             println!("write_interleaved {:?}", err);
    //         };
    //     }
    // }
}

// TODO: Remove debug function
pub fn _save_file(frame: &Video, index: usize) -> std::result::Result<(), std::io::Error> {
    let mut file = File::create(format!("frame{}.ppm", index))?;
    file.write_all(format!("P6\n{} {}\n255\n", frame.width(), frame.height()).as_bytes())?;
    file.write_all(frame.data(0))?;
    Ok(())
}
