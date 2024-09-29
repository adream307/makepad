use makepad_widgets::*;
        
live_design!{
    import makepad_widgets::base::*;
    import makepad_widgets::theme_desktop_dark::*; 
    
    App = {{App}} {
        ui: <Root>{
            main_window = <Window>{
                show_bg: true
                width: Fill,
                height: Fill
                body = <ScrollXYView>{
                    flow: Down,
                    spacing:10,
                    align: {
                        x: 0.5,
                        y: 0.5
                    },
                    <Label> {
                        draw_text: {
                            text_style: {
                                font_size: 70
                            },
                        }
                        text: "A"
                    }
                    <Label> {
                        draw_text: {
                            text_style: {
                                font_size: 70
                            },
                        }
                        text: "B"
                    }
                    <Label> {
                        draw_text: {
                            text_style: {
                                font_size: 70
                            },
                        }
                        text: "C"
                    }
                    <Label> {
                        draw_text: {
                            text_style: {
                                font_size: 70
                            },
                        }
                        text: "D"
                    }
                }
            }
        }
    }
}  
              
app_main!(App); 
 
#[derive(Live, LiveHook)]
pub struct App {
    #[live] ui: WidgetRef,
    #[rust] counter: usize,
 }
 
impl LiveRegister for App {
    fn live_register(cx: &mut Cx) {
        crate::makepad_widgets::live_design(cx);
    }
}

impl MatchEvent for App{}

impl AppMain for App {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        self.match_event(cx, event);
        self.ui.handle_event(cx, event, &mut Scope::empty());
    }
}
