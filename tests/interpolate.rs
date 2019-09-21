//! Tests for the `Converter` and `Interpolator` traits

extern crate sample;

#[cfg(feature="interpolate")]
use sample::interpolate::{Converter, Floor, Linear, Sinc};
#[cfg(feature="ring_buffer")]
use sample::ring_buffer;
#[cfg(feature="signal")]
use sample::{signal, Signal};

#[cfg(feature="interpolate")]
#[test]
fn test_floor_converter() {
    let frames: [[f64; 1]; 3] = [[0.0], [1.0], [2.0]];
    let mut source = signal::from_iter(frames.iter().cloned());
    let interp = Floor::from_source(&mut source);
    let mut conv = Converter::scale_playback_hz(source, interp, 0.5);

    assert_eq!(conv.next(), [0.0]);
    assert_eq!(conv.next(), [0.0]);
    assert_eq!(conv.next(), [1.0]);
    assert_eq!(conv.next(), [1.0]);
    // It may seem odd that we are emitting two values, but consider this: no matter what the next
    // value would be, Floor would always yield the same frame until we hit an interpolation_value
    // of 1.0 and had to advance the frame. We don't know what the future holds, so we should
    // continue yielding frames.
    assert_eq!(conv.next(), [2.0]);
    assert_eq!(conv.next(), [2.0]);
}

#[cfg(all(feature="interpolate", feature = "signal"))]
#[test]
fn test_linear_converter() {
    let frames: [[f64; 1]; 3] = [[0.0], [1.0], [2.0]];
    let mut source = signal::from_iter(frames.iter().cloned());
    let interp = Linear::from_source(&mut source);
    let mut conv = Converter::scale_playback_hz(source, interp, 0.5);

    assert_eq!(conv.next(), [0.0]);
    assert_eq!(conv.next(), [0.5]);
    assert_eq!(conv.next(), [1.0]);
    assert_eq!(conv.next(), [1.5]);
    assert_eq!(conv.next(), [2.0]);
    // There's nothing else here to interpolate toward, but we do want to ensure that we're
    // emitting the correct number of frames.
    assert_eq!(conv.next(), [1.0]);
}

#[cfg(all(feature="interpolate", feature = "signal"))]
#[test]
fn test_scale_playback_rate() {
    // Scale the playback rate by `0.5`
    let foo = [[0.0], [1.0], [0.0], [-1.0]];
    let mut source = signal::from_iter(foo.iter().cloned());
    let interp = Linear::from_source(&mut source);
    let frames: Vec<_> = source.scale_hz(interp, 0.5).take(8).collect();
    assert_eq!(
        &frames[..],
        &[[0.0], [0.5], [1.0], [0.5], [0.0], [-0.5], [-1.0], [-0.5]][..]
    );
}

#[cfg(all(feature="interpolate", feature = "signal"))]
#[test]
fn test_sinc() {
    let foo = [[0.0f64], [1.0], [0.0], [-1.0]];
    let source = signal::from_iter(foo.iter().cloned());

    let frames = ring_buffer::Fixed::from(vec![[0.0]; 50]);
    let interp = Sinc::new(frames);
    let resampled = source.from_hz_to_hz(interp, 44100.0, 11025.0);

    assert_eq!(
        resampled.until_exhausted().find(|sample| sample[0].is_nan()),
        None
    );
}
