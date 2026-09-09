//! Helpers for constructing digital stimulus waveforms.

use rust_decimal::Decimal;
use substrate::simulation::waveform::{TimeWaveform, Waveform};

use crate::bits::{is_logical_high, is_logical_low, BitSignal};
use crate::sim::dec;

/// Extension methods for building clocked digital stimulus as an `f64` waveform.
pub trait DigitalWaveform {
    /// Holds the waveform high (at `vdd`) until time `until`.
    ///
    /// If the waveform is currently low, a rising transition of duration `tr` is inserted
    /// immediately after the last point.
    fn push_high(&mut self, until: f64, vdd: f64, tr: f64);

    /// Holds the waveform low until time `until`.
    ///
    /// If the waveform is currently high, a falling transition of duration `tf` is inserted
    /// immediately after the last point.
    fn push_low(&mut self, until: f64, vdd: f64, tf: f64);

    /// Pushes a high level if `bit` is set, or a low level otherwise.
    fn push_bit(&mut self, bit: bool, until: f64, vdd: f64, t_transition: f64) {
        if bit {
            self.push_high(until, vdd, t_transition);
        } else {
            self.push_low(until, vdd, t_transition);
        }
    }

    /// Converts this waveform to a decimal-valued waveform suitable for a PWL source.
    fn to_decimal(&self) -> Waveform<Decimal>;
}

impl DigitalWaveform for Waveform<f64> {
    fn push_high(&mut self, until: f64, vdd: f64, tr: f64) {
        if self.is_empty() {
            *self = [(until, vdd)].into_iter().collect();
            return;
        }
        if let Some(t) = self.last_t() {
            assert!(until > t);
        }
        if is_logical_low(self.last_x().unwrap_or(vdd), vdd) {
            self.push(self.last_t().unwrap() + tr, vdd);
        }
        self.push(until, vdd);
    }

    fn push_low(&mut self, until: f64, vdd: f64, tf: f64) {
        if self.is_empty() {
            *self = [(until, 0.)].into_iter().collect();
            return;
        }
        if let Some(t) = self.last_t() {
            assert!(until > t);
        }
        if is_logical_high(self.last_x().unwrap_or(0f64), vdd) {
            self.push(self.last_t().unwrap() + tf, 0f64);
        }
        self.push(until, 0f64);
    }

    fn to_decimal(&self) -> Waveform<Decimal> {
        self.values().map(|pt| (dec(pt.t()), dec(pt.x()))).collect()
    }
}

/// Pushes one bit of `signal` onto each waveform in `waveforms`, holding until `until`.
pub fn push_bus(
    waveforms: &mut [Waveform<f64>],
    signal: &BitSignal,
    until: f64,
    vdd: f64,
    tr: f64,
    tf: f64,
) {
    assert_eq!(waveforms.len(), signal.width());
    for (i, bit) in signal.bits().enumerate() {
        if bit {
            waveforms[i].push_high(until, vdd, tr);
        } else {
            waveforms[i].push_low(until, vdd, tf);
        }
    }
}

/// Returns the index of the last point in `w` whose time is at most `t`.
pub fn index_before<W: TimeWaveform<Data = f64>>(w: &W, t: f64) -> Option<usize> {
    if t.is_nan() || t < w.first_t()? {
        return None;
    }
    w.time_index_before(t)
}

/// Digital stimulus sampled in seconds and volts.
pub type DigitalTrace = Waveform<f64>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_stimulus_and_times_before_first_sample() {
        let mut waveform = Waveform::new();
        assert_eq!(index_before(&waveform, 0.), None);
        waveform.push_high(1., 1.8, 0.1);
        assert_eq!(waveform.len(), 1);
        assert_eq!(index_before(&waveform, 0.5), None);
        assert_eq!(index_before(&waveform, 1.), Some(0));
        waveform.push_low(2., 1.8, 0.1);
        assert_eq!(waveform.len(), 3);
        assert_eq!(waveform.last_x(), Some(0.));
        assert_eq!(waveform.to_decimal().len(), waveform.len());
    }
}
