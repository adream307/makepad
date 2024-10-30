use makepad_widgets::*;
   
live_design!{ 
    import makepad_widgets::base::*;
    import makepad_widgets::theme_desktop_dark::*;
    import makepad_draw::shader::std::*;


    NewsFeed ={{NewsFeed}}{
        list = <PortalList>{
            btn_red=<CachedView>{
                height:180,
                width:250,
                draw_bg:{ fn pixel(self) -> vec4 { return (#xf00) } }
            }
            btn_black=<CachedView>{
                height:180,
                width:250,
                draw_bg:{ fn pixel(self) -> vec4 { return (#x0) } }
            }
        }
    }

    App = {{App}} {
        ui: <Window> {
            
            window: {inner_size: vec2(300, 600)},
            show_bg: true
            draw_bg: {
                fn pixel(self) -> vec4 {
                    return (#xfff8ee);
                }
            }
            body = {
                flow: Overlay,
                padding: 0.0
                spacing: 0,
                align: {
                    x: 0.0,
                    y: 0.0
                },
                news_feed = <NewsFeed>{}

            }
        }
    }
}

app_main!(App);

#[derive(Live, LiveHook, Widget)]
struct NewsFeed{ 
    #[deref] view:View
}

impl Widget for NewsFeed{
    fn draw_walk(&mut self, cx:&mut Cx2d, scope:&mut Scope, walk:Walk)->DrawStep{
        log!("========= draw_walk");
        while let Some(item) =  self.view.draw_walk(cx, scope, walk).step(){
            if let Some(mut list) = item.as_portal_list().borrow_mut() {
                list.set_item_range(cx, 0, 1000);
                while let Some(item_id) = list.next_visible_item(cx) {
                    log!("========== item {item_id}");
                    let temp_id = if item_id%2==0 { live_id!(btn_red)} else {live_id!(btn_black)};
                    let item = list.item(cx, item_id, temp_id);
                    //item.as_label().set_text(&format!("{item_id}"));
                    item.draw_all(cx, &mut Scope::empty());
                }
            }
        }
        DrawStep::done()
    }
    fn handle_event(&mut self, cx:&mut Cx, event:&Event, scope:&mut Scope){
        self.view.handle_event(cx, event, scope)
    }
}

#[derive(Live, LiveHook)]
pub struct App {
    #[live] ui: WidgetRef,
}

impl LiveRegister for App {
    fn live_register(cx: &mut Cx) {
        crate::makepad_widgets::live_design(cx);
    } 
}

impl MatchEvent for App {
    fn handle_actions(&mut self, _cx:&mut Cx, actions:&Actions){
    }
}

impl AppMain for App {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        self.match_event(cx, event);
        self.ui.handle_event(cx, event, &mut Scope::empty());
    }
}
