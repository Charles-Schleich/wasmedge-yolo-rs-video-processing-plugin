ffmpeg -loop 1 \
    -i demo_image.png \
    -c:v libx264 \
    -t 5 \
    -pix_fmt rgb24 \
    -vf scale=2:2 \
    -r 30 \
    out.mp4
