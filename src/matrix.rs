// Matrix keypad module
use core::array::from_fn;
use core::convert::Infallible;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal;
use embassy_time::Timer;
use embedded_hal::digital::{ErrorType, InputPin, OutputPin};
use embedded_hal_async::digital::Wait;

pub(crate) struct KeyPin<'a> {
    signal: &'a signal::Signal<CriticalSectionRawMutex, bool>,
    state: bool,
}

impl<'a> KeyPin<'a> {
    pub(crate) fn new(signal: &'a signal::Signal<CriticalSectionRawMutex, bool>) -> Self {
        Self {
            state: false,
            signal,
        }
    }

    async fn wait_for_value(&mut self) -> bool {
        self.state = self.signal.wait().await;
        self.state
    }

    pub(crate) async fn wait_for_state(&mut self, state: bool) -> Result<(), Infallible> {
        let mut cur_state = self.is_high().unwrap();
        while cur_state != state {
            cur_state = self.wait_for_value().await;
        }
        Ok(())
    }

    pub(crate) async fn wait_for_edge(&mut self) -> bool {
        let _ = self.signal.try_take();
        self.wait_for_value().await
    }
}

impl ErrorType for KeyPin<'_> {
    type Error = Infallible;
}

impl InputPin for KeyPin<'_> {
    fn is_high(&mut self) -> Result<bool, Self::Error> {
        let new_state = self.signal.try_take();
        self.state = new_state.unwrap_or(self.state);
        Ok(self.state)
    }

    fn is_low(&mut self) -> Result<bool, Self::Error> {
        let new_state = self.signal.try_take();
        self.state = new_state.unwrap_or(self.state);
        Ok(!self.state)
    }
}

impl Wait for KeyPin<'_> {
    async fn wait_for_high(&mut self) -> Result<(), Self::Error> {
        self.wait_for_state(true).await
    }

    async fn wait_for_low(&mut self) -> Result<(), Self::Error> {
        self.wait_for_state(false).await
    }

    async fn wait_for_rising_edge(&mut self) -> Result<(), Self::Error> {
        while !(self.wait_for_edge().await) {}
        Ok(())
    }

    async fn wait_for_falling_edge(&mut self) -> Result<(), Self::Error> {
        while self.wait_for_edge().await {}
        Ok(())
    }

    async fn wait_for_any_edge(&mut self) -> Result<(), Self::Error> {
        self.wait_for_edge().await;
        Ok(())
    }
}

struct KeyState<'a> {
    signal: Option<&'a signal::Signal<CriticalSectionRawMutex, bool>>,
    state: bool,
}

impl KeyState<'_> {
    fn new() -> Self {
        Self {
            signal: None,
            state: false,
        }
    }

    fn set_value(&mut self, state: bool) {
        if state != self.state {
            let _ = self.signal.inspect(|s| s.signal(state));
        }
        self.state = state;
    }
}

pub(crate) struct Matrix<
    'a,
    In: Wait + InputPin,
    Out: OutputPin,
    const INPUT_PIN_NUM: usize,
    const OUTPUT_PIN_NUM: usize,
> {
    input_pins: [In; INPUT_PIN_NUM],
    output_pins: [Out; OUTPUT_PIN_NUM],
    key_states: [[KeyState<'a>; INPUT_PIN_NUM]; OUTPUT_PIN_NUM],
}

impl<
        'a,
        In: Wait + InputPin,
        Out: OutputPin,
        const INPUT_PIN_NUM: usize,
        const OUTPUT_PIN_NUM: usize,
    > Matrix<'a, In, Out, INPUT_PIN_NUM, OUTPUT_PIN_NUM>
{
    pub(crate) fn new(input_pins: [In; INPUT_PIN_NUM], output_pins: [Out; OUTPUT_PIN_NUM]) -> Self {
        Self {
            input_pins,
            output_pins,
            key_states: from_fn::<_, OUTPUT_PIN_NUM, _>(|_| {
                from_fn::<_, INPUT_PIN_NUM, _>(|_| KeyState::new())
            }),
        }
    }

    async fn scan_row(out_pin: &mut Out, input_pins: &mut [In], key_states: &mut [KeyState<'_>]) {
        out_pin.set_low().ok();
        Timer::after_micros(1).await;
        for (in_idx, in_pin) in input_pins.iter_mut().enumerate() {
            let state = in_pin.is_high().unwrap_or_default();
            key_states[in_idx].set_value(state);
        }
        out_pin.set_high().ok();
    }

    pub(crate) async fn scan(&mut self) {
        for (out_idx, out_pin) in self.output_pins.iter_mut().enumerate() {
            let inputs = self.input_pins.as_mut();
            let key_states = self.key_states[out_idx].as_mut();
            Self::scan_row(out_pin, inputs, key_states).await;
        }
    }

    pub(crate) fn take_pin<'b, 'c>(
        &mut self,
        row: usize,
        col: usize,
        s: &'b signal::Signal<CriticalSectionRawMutex, bool>,
    ) -> KeyPin<'c>
    where
        'b: 'a,
        'b: 'c,
    {
        self.key_states[row][col].signal = Some(s);
        KeyPin::new(s)
    }
}
