use crate::event::{
    KeyEvent, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ScrollEvent, TextInputEvent, TimerEvent,
};

#[derive(Debug)]
pub enum OpenHarmonyEvent {
    Paint,
    MouseDown(MouseDownEvent),
    MouseUp(MouseUpEvent),
    MouseMove(MouseMoveEvent),
    Scroll(ScrollEvent),
    KeyDown(KeyEvent),
    KeyUp(KeyEvent),
    TextInput(TextInputEvent),
    Timer(TimerEvent),
}
