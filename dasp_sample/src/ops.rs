pub mod f32 {
    #[allow(unused_imports)]
    use core;

    #[cfg(not(feature = "std"))]
    pub fn sqrt(x: f32) -> f32 {
        num_traits::Float::sqrt(x)
    }
    #[cfg(feature = "std")]
    pub fn sqrt(x: f32) -> f32 {
        x.sqrt()
    }
}

pub mod f64 {
    #[allow(unused_imports)]
    use core;

    #[cfg(not(feature = "std"))]
    pub fn sqrt(x: f64) -> f64 {
        num_traits::Float::sqrt(x)
    }
    #[cfg(feature = "std")]
    pub fn sqrt(x: f64) -> f64 {
        x.sqrt()
    }
}
