use image::{ImageBuffer, Rgb, RgbImage};

mod colors;
mod nibbler;
pub mod utils;

const SPRITE_SIZE: u32 = 5;
const IMAGE_SIZE: u32 = 290;
const PIXEL_SIZE: u32 = IMAGE_SIZE / (SPRITE_SIZE + 1);
const MARGIN: u32 = PIXEL_SIZE / 2;

pub fn gen(data: &[u8]) -> RgbImage {
    let hash = utils::md5(data);

    let background = Rgb([240, 240, 240]);
    let foreground = colors::DARK_COLORS
        [(hash[11] as usize + hash[12] as usize + hash[15] as usize) % colors::DARK_COLORS.len()];

    let mut image: RgbImage = ImageBuffer::from_pixel(IMAGE_SIZE, IMAGE_SIZE, background);

    for (row, pix) in pixels(hash).chunks(SPRITE_SIZE as usize).enumerate() {
        for (col, painted) in pix.iter().enumerate() {
            if *painted {
                let x = col as u32 * PIXEL_SIZE;
                let y = row as u32 * PIXEL_SIZE;
                draw_rect(
                    &mut image,
                    x + MARGIN,
                    y + MARGIN,
                    x + PIXEL_SIZE + MARGIN,
                    y + PIXEL_SIZE + MARGIN,
                    foreground,
                );
            }
        }
    }

    image
}

fn pixels(hash: [u8; 16]) -> [bool; 25] {
    let mut nibbles = nibbler::Nibbler::new(&hash).map(|x| x % 2 == 0);
    let mut pixels = [false; 25];
    for col in (0..3).rev() {
        for row in 0..5 {
            let ix = col + (row * 5);
            let mirror_col = 4 - col;
            let mirror_ix = mirror_col + (row * 5);
            let paint = nibbles.next().unwrap();
            pixels[ix] = paint;
            pixels[mirror_ix] = paint;
        }
    }
    pixels
}

fn draw_rect(image: &mut RgbImage, x0: u32, y0: u32, x1: u32, y1: u32, color: Rgb<u8>) {
    for x in x0..x1 {
        for y in y0..y1 {
            image.put_pixel(x, y, color);
        }
    }
}
