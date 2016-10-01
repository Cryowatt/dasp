use {Duplex, Frame, Sample};

/// An iterator that converts the rate at which frames are yielded from some given frame
/// Interpolator into a new type.
///
/// Other names for `sample::rate::Converter` might include:
///
/// - Sample rate converter
/// - {Up/Down}sampler
/// - Sample interpolater
/// - Sample decimator
///
#[derive(Clone)]
pub struct Converter<T: Iterator, I: Interpolator>
    where <T as Iterator>::Item: Frame
{
    source: T,
    interpolator: I,
    interpolation_value: f64,
    source_to_target_ratio: f64
}

/// Interpolator that just rounds off any values to the previous value from the source
pub struct Floor<F>
{
    left: F
}

/// Interpolator that interpolates linearly between the previous value and the next value
pub struct Linear<F>
{
    left: F,
    right: Option<F>
}

/// Trait for all things that can interpolate between two values. Implementations should keep track
/// of the necessary data both before and after the current frame.
pub trait Interpolator
{
    type Frame: Frame;

    /// Given a distance between [0. and 1.) to the following sample, return the interpolated value
    fn interpolate(&self, x: f64) -> Self::Frame;

    /// Called whenever the Interpolator value is over 1.
    fn next_source_frame(&mut self, source_frame: Option<Self::Frame>);
}

impl<T, I> Converter<T, I> 
    where T: Iterator,
          <T as Iterator>::Item: Frame,
          I: Interpolator
{
    /// Construct a new `Converter` from the source frames and the source and target sample rates
    /// (in Hz).
    #[inline]
    pub fn from_hz_to_hz(source: T, interpolator: I, source_hz: f64, target_hz: f64) -> Self {
        Self::scale_playback_hz(source, interpolator, source_hz / target_hz)
    }

    /// Construct a new `Converter` from the source frames and the amount by which the current
    /// ***playback*** **rate** (not sample rate) should be multiplied to reach the new playback
    /// rate.
    ///
    /// For example, if our `source_frames` is a sine wave oscillating at a frequency of 2hz and
    /// we wanted to convert it to a frequency of 3hz, the given `scale` should be `1.5`.
    #[inline]
    pub fn scale_playback_hz(source: T, interpolator: I, scale: f64) -> Self {
        assert!(scale > 0.0, "We can't yield any frames at 0 times a second!");
        Converter {
            source: source,
            interpolator: interpolator,
            interpolation_value: 0.0,
            source_to_target_ratio: scale
        }
    }

    /// Construct a new `Converter` from the source frames and the amount by which the current
    /// ***sample*** **rate** (not playback rate) should be multiplied to reach the new sample
    /// rate.
    ///
    /// If our `source_frames` are being sampled at a rate of 44_100hz and we want to
    /// convert to a sample rate of 96_000hz, the given `scale` should be `96_000.0 / 44_100.0`.
    ///
    /// This is the same as calling `Converter::scale_playback_hz(source_frames, 1.0 / scale)`.
    #[inline]
    pub fn scale_sample_hz(source: T, interpolator: I, scale: f64) -> Self {
        Self::scale_playback_hz(source, interpolator, 1.0 / scale)
    }

    /// Update the `source_to_target_ratio` internally given the source and target hz.
    ///
    /// This method might be useful for changing the sample rate during playback.
    #[inline]
    pub fn set_hz_to_hz(&mut self, source_hz: f64, target_hz: f64) {
        self.set_playback_hz_scale(source_hz / target_hz)
    }

    /// Update the `source_to_target_ratio` internally given a new **playback rate** multiplier.
    ///
    /// This method is useful for dynamically changing rates.
    #[inline]
    pub fn set_playback_hz_scale(&mut self, scale: f64) {
        self.source_to_target_ratio = scale;
    }

    /// Update the `source_to_target_ratio` internally given a new **sample rate** multiplier.
    ///
    /// This method is useful for dynamically changing rates.
    #[inline]
    pub fn set_sample_hz_scale(&mut self, scale: f64) {
        self.set_playback_hz_scale(1.0 / scale);
    }

    /// Borrow the `source_frames` Interpolator from the `Converter`.
    #[inline]
    pub fn source(&self) -> &T {
        &self.source
    }

    /// Mutably borrow the `source_frames` Iterator from the `Converter`.
    #[inline]
    pub fn source_mut(&mut self) -> &mut T {
        &mut self.source
    }

    /// Drop `self` and return the internal `source_frames` Iterator.
    #[inline]
    pub fn into_source(self) -> T {
        self.source
    }
}

impl<T, I> Iterator for Converter<T, I> 
    where T: Iterator,
          <T as Iterator>::Item: Frame,
          <<T as Iterator>::Item as Frame>::Sample: Duplex<f64>,
          I: Interpolator<Frame=<T as Iterator>::Item>
{
    type Item = <T as Iterator>::Item;

    fn next(&mut self) -> Option<Self::Item> {
        let Converter {
            ref mut source, 
            ref mut interpolator,
            ref mut interpolation_value,
            source_to_target_ratio
        } = *self;
 
        while *interpolation_value >= 1.0 {
            interpolator.next_source_frame(source.next());
            *interpolation_value -= 1.0;
        }

        let out = Some(interpolator.interpolate(*interpolation_value));
        *interpolation_value += source_to_target_ratio;

        out
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len_multiplier = self.source_to_target_ratio / 1.0;
        let (source_lower, source_upper) = self.source.size_hint();
        let lower = (source_lower as f64 * len_multiplier) as usize;
        let upper = source_upper.map(|upper| (upper as f64 * len_multiplier) as usize);
        (lower, upper)
    }
}

impl<F> Interpolator for Floor<F>
    where F: Frame,
          <F as Frame>::Sample: Duplex<f64>
{
    type Frame = F;

    fn interpolate(&self, _x: f64) -> Self::Frame {
        self.left
    }

    fn next_source_frame(&mut self, source_frame: Option<Self::Frame>) {
        self.left = source_frame.unwrap_or(self.left);
    }
}

impl<F> Interpolator for Linear<F>
    where F: Frame,
          <F as Frame>::Sample: Duplex<f64>
{
    type Frame = F;

    /// Converts linearly from the previous value, using the next value to interpolate. It is
    /// possible, although not advisable, to provide an x > 1.0 or < 0.0, but this will just
    /// continue to be a linear ramp in one direction or another.
    fn interpolate(&self, x: f64) -> Self::Frame {
        let left = self.left;
        match self.right {
            Some(right) => {
                self.left.zip_map(right, |l, r| {
                    let l_f = l.to_sample::<f64>();
                    let r_f = r.to_sample::<f64>();
                    let diff = r_f - l_f;
                    ((diff * x) + l_f).to_sample::<<Self::Frame as Frame>::Sample>()
                })
            }
            None => left
        }
    }

    fn next_source_frame(&mut self, source_frame: Option<Self::Frame>) {
        self.left = self.right.unwrap_or(self.left);
        self.right = source_frame;
    }
}

