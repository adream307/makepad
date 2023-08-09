use {
    crate::{
        char::CharExt,
        line::Wrapped,
        selection::Affinity,
        state::{Block, Session},
        str::StrExt,
        token::TokenKind,
        Line, Point, Selection, Token,
    },
    makepad_widgets::*,
    std::{mem, slice::Iter},
};

live_design! {
    import makepad_draw::shader::std::*;
    import makepad_widgets::theme::*;

    DrawSelection = {{DrawSelection}} {
        uniform gloopiness: 8.0
        uniform border_radius: 2.0

        fn vertex(self) -> vec4 {
            let clipped: vec2 = clamp(
                self.geom_pos * vec2(self.rect_size.x + 16., self.rect_size.y) + self.rect_pos - vec2(8., 0.),
                self.draw_clip.xy,
                self.draw_clip.zw
            );
            self.pos = (clipped - self.rect_pos) / self.rect_size;
            return self.camera_projection * (self.camera_view * (
                self.view_transform * vec4(clipped.x, clipped.y, self.draw_depth + self.draw_zbias, 1.)
            ));
        }

        fn pixel(self) -> vec4 {
            let sdf = Sdf2d::viewport(self.rect_pos + self.pos * self.rect_size);
            sdf.box(
                self.rect_pos.x,
                self.rect_pos.y,
                self.rect_size.x,
                self.rect_size.y,
                self.border_radius
            );
            if self.prev_w > 0.0 {
                sdf.box(
                    self.prev_x,
                    self.rect_pos.y - self.rect_size.y,
                    self.prev_w,
                    self.rect_size.y,
                    self.border_radius
                );
                sdf.gloop(self.gloopiness);
            }
            if self.next_w > 0.0 {
                sdf.box(
                    self.next_x,
                    self.rect_pos.y + self.rect_size.y,
                    self.next_w,
                    self.rect_size.y,
                    self.border_radius
                );
                sdf.gloop(self.gloopiness);
            }
            return sdf.fill(#08f8);
        }
    }

    CodeEditor = {{CodeEditor}} {
        walk: {
            width: Fill,
            height: Fill,
            margin: 0,
        },
        draw_text: {
            draw_depth: 0.0,
            text_style: <FONT_CODE> {}
        }
        draw_selection: {
            draw_depth: 1.0,
        }
        draw_cursor: {
            draw_depth: 2.0,
            color: #C0C0C0,
        }
    }
}

#[derive(Live, LiveHook)]
pub struct CodeEditor {
    #[live]
    scroll_bars: ScrollBars,
    #[live]
    walk: Walk,
    #[live]
    draw_text: DrawText,
    #[live]
    draw_selection: DrawSelection,
    #[live]
    draw_cursor: DrawColor,
    #[rust]
    viewport_rect: Rect,
    #[rust]
    cell_size: DVec2,
    #[rust]
    start: usize,
    #[rust]
    end: usize,
}

impl CodeEditor {
    pub fn draw(&mut self, cx: &mut Cx2d<'_>, session: &mut Session) {
        self.viewport_rect = Rect {
            pos: self.scroll_bars.get_scroll_pos(),
            size: cx.turtle().rect().size,
        };
        self.cell_size =
            self.draw_text.text_style.font_size * self.draw_text.get_monospace_base(cx);
        session.handle_changes();
        session.set_wrap_column(Some(
            (self.viewport_rect.size.x / self.cell_size.x) as usize,
        ));
        self.start =
            session.find_first_line_ending_after_y(self.viewport_rect.pos.y / self.cell_size.y);
        self.end = session.find_first_line_starting_after_y(
            (self.viewport_rect.pos.y + self.viewport_rect.size.y) / self.cell_size.y,
        );
        self.scroll_bars.begin(cx, self.walk, Layout::default());
        self.draw_text(cx, session);
        self.draw_selections(cx, session);
        cx.turtle_mut().set_used(
            session.width() * self.cell_size.x,
            session.height() * self.cell_size.y,
        );
        self.scroll_bars.end(cx);
    }

    pub fn handle_event(&mut self, cx: &mut Cx, session: &mut Session, event: &Event) {
        session.handle_changes();
        self.scroll_bars.handle_event_with(cx, event, &mut |cx, _| {
            cx.redraw_all();
        });
        match event {
            Event::TextInput(TextInputEvent { input, .. }) => {
                session.insert(input.into());
                cx.redraw_all();
            }
            Event::KeyDown(KeyEvent {
                key_code: KeyCode::ReturnKey,
                ..
            }) => {
                session.enter();
                cx.redraw_all();
            }
            Event::KeyDown(KeyEvent {
                key_code: KeyCode::Delete,
                ..
            }) => {
                session.delete();
                cx.redraw_all();
            }
            Event::KeyDown(KeyEvent {
                key_code: KeyCode::Backspace,
                ..
            }) => {
                session.backspace();
                cx.redraw_all();
            }
            _ => {}
        }
        match event.hits(cx, self.scroll_bars.area()) {
            Hit::FingerDown(FingerDownEvent {
                abs,
                rect,
                modifiers: KeyModifiers { alt, .. },
                ..
            }) => {
                if let Some((cursor, affinity)) = self.pick(session, abs - rect.pos) {
                    if alt {
                        session.add_cursor(cursor, affinity);
                    } else {
                        session.set_cursor(cursor, affinity);
                    }
                    cx.redraw_all();
                }
            }
            Hit::FingerMove(FingerMoveEvent { abs, rect, .. }) => {
                if let Some((cursor, affinity)) = self.pick(session, abs - rect.pos) {
                    session.move_to(cursor, affinity);
                    cx.redraw_all();
                }
            }
            _ => {}
        }
    }

    fn draw_text(&mut self, cx: &mut Cx2d<'_>, session: &Session) {
        let mut y = 0.0;
        session.blocks(
            0,
            session.document().borrow().text().as_lines().len(),
            |blocks| {
                for block in blocks {
                    match block {
                        Block::Line { line, .. } => {
                            let mut token_iter = line.tokens().iter().copied();
                            let mut token_slot = token_iter.next();
                            let mut column = 0;
                            for wrapped in line.wrappeds() {
                                match wrapped {
                                    Wrapped::Text {
                                        is_inlay: false,
                                        mut text,
                                    } => {
                                        while !text.is_empty() {
                                            let token = match token_slot {
                                                Some(token) => {
                                                    if text.len() < token.len {
                                                        token_slot = Some(Token {
                                                            len: token.len - text.len(),
                                                            kind: token.kind,
                                                        });
                                                        Token {
                                                            len: text.len(),
                                                            kind: token.kind,
                                                        }
                                                    } else {
                                                        token_slot = token_iter.next();
                                                        token
                                                    }
                                                }
                                                None => Token {
                                                    len: text.len(),
                                                    kind: TokenKind::Unknown,
                                                },
                                            };
                                            let (text_0, text_1) = text.split_at(token.len);
                                            text = text_1;
                                            self.draw_text.draw_abs(
                                                cx,
                                                DVec2 {
                                                    x: line.column_to_x(column),
                                                    y,
                                                } * self.cell_size
                                                    - self.viewport_rect.pos,
                                                text_0,
                                            );
                                            column += text_0
                                                .chars()
                                                .map(|char| {
                                                    char.column_count(
                                                        session.settings().tab_column_count,
                                                    )
                                                })
                                                .sum::<usize>();
                                        }
                                    }
                                    Wrapped::Text {
                                        is_inlay: true,
                                        text,
                                    } => {
                                        self.draw_text.draw_abs(
                                            cx,
                                            DVec2 {
                                                x: line.column_to_x(column),
                                                y,
                                            } * self.cell_size
                                                - self.viewport_rect.pos,
                                            text,
                                        );
                                        column += text
                                            .chars()
                                            .map(|char| {
                                                char.column_count(
                                                    session.settings().tab_column_count,
                                                )
                                            })
                                            .sum::<usize>();
                                    }
                                    Wrapped::Widget(widget) => {
                                        column += widget.column_count;
                                    }
                                    Wrapped::Wrap => {
                                        column = line.wrap_indent_column_count();
                                        y += line.scale();
                                    }
                                }
                            }
                            y += line.scale();
                        }
                        Block::Widget(widget) => {
                            y += widget.height;
                        }
                    }
                }
            },
        );
    }

    fn draw_selections(&mut self, cx: &mut Cx2d<'_>, session: &Session) {
        let mut active_selection = None;
        let mut selections = session.selections().iter();
        while selections
            .as_slice()
            .first()
            .map_or(false, |selection| selection.end().line < self.start)
        {
            selections.next().unwrap();
        }
        if selections
            .as_slice()
            .first()
            .map_or(false, |selection| selection.start().line < self.start)
        {
            active_selection = Some(ActiveSelection {
                selection: *selections.next().unwrap(),
                start_x: 0.0,
            });
        }
        DrawSelections {
            code_editor: self,
            active_selection,
            selections,
        }
        .draw_selections(cx, session)
    }

    fn pick(&self, session: &Session, point: DVec2) -> Option<(Point, Affinity)> {
        let point = (point + self.viewport_rect.pos) / self.cell_size;
        let mut line = session.find_first_line_ending_after_y(point.y);
        let mut y = session.line(line, |line| line.y());
        session.blocks(line, line + 1, |blocks| {
            for block in blocks {
                match block {
                    Block::Line {
                        is_inlay: false,
                        line: line_ref,
                    } => {
                        let mut byte = 0;
                        let mut column = 0;
                        for wrapped in line_ref.wrappeds() {
                            match wrapped {
                                Wrapped::Text {
                                    is_inlay: false,
                                    text,
                                } => {
                                    for grapheme in text.graphemes() {
                                        let next_byte = byte + grapheme.len();
                                        let next_column = column
                                            + grapheme
                                                .chars()
                                                .map(|char| {
                                                    char.column_count(
                                                        session.settings().tab_column_count,
                                                    )
                                                })
                                                .sum::<usize>();
                                        let next_y = y + line_ref.scale();
                                        let x = line_ref.column_to_x(column);
                                        let next_x = line_ref.column_to_x(next_column);
                                        let mid_x = (x + next_x) / 2.0;
                                        if (y..=next_y).contains(&point.y) {
                                            if (x..=mid_x).contains(&point.x) {
                                                return Some((
                                                    Point { line, byte },
                                                    Affinity::After,
                                                ));
                                            }
                                            if (mid_x..=next_x).contains(&point.x) {
                                                return Some((
                                                    Point {
                                                        line,
                                                        byte: next_byte,
                                                    },
                                                    Affinity::Before,
                                                ));
                                            }
                                        }
                                        byte = next_byte;
                                        column = next_column;
                                    }
                                }
                                Wrapped::Text {
                                    is_inlay: true,
                                    text,
                                } => {
                                    let next_column = column
                                        + text
                                            .chars()
                                            .map(|char| {
                                                char.column_count(
                                                    session.settings().tab_column_count,
                                                )
                                            })
                                            .sum::<usize>();
                                    let next_y = y + line_ref.scale();
                                    let x = line_ref.column_to_x(column);
                                    let next_x = line_ref.column_to_x(next_column);
                                    if (y..=next_y).contains(&point.y)
                                        && (x..=next_x).contains(&point.x)
                                    {
                                        return Some((Point { line, byte }, Affinity::Before));
                                    }
                                    column = next_column;
                                }
                                Wrapped::Widget(widget) => {
                                    column += widget.column_count;
                                }
                                Wrapped::Wrap => {
                                    let next_y = y + line_ref.scale();
                                    if (y..=next_y).contains(&point.y) {
                                        return Some((Point { line, byte }, Affinity::Before));
                                    }
                                    column = line_ref.wrap_indent_column_count();
                                    y = next_y;
                                }
                            }
                        }
                        let next_y = y + line_ref.scale();
                        if (y..=y + next_y).contains(&point.y) {
                            return Some((Point { line, byte }, Affinity::After));
                        }
                        line += 1;
                        y = next_y;
                    }
                    Block::Line {
                        is_inlay: true,
                        line: line_ref,
                    } => {
                        let next_y = y + line_ref.height();
                        if (y..=next_y).contains(&point.y) {
                            return Some((Point { line, byte: 0 }, Affinity::Before));
                        }
                        y = next_y;
                    }
                    Block::Widget(widget) => {
                        y += widget.height;
                    }
                }
            }
            None
        })
    }
}

struct DrawSelections<'a> {
    code_editor: &'a mut CodeEditor,
    active_selection: Option<ActiveSelection>,
    selections: Iter<'a, Selection>,
}

impl<'a> DrawSelections<'a> {
    fn draw_selections(&mut self, cx: &mut Cx2d<'_>, session: &Session) {
        let mut line = self.code_editor.start;
        let mut y = session.line(line, |line| line.y());
        session.blocks(self.code_editor.start, self.code_editor.end, |blocks| {
            for block in blocks {
                match block {
                    Block::Line {
                        is_inlay: false,
                        line: line_ref,
                    } => {
                        let mut byte = 0;
                        let mut column = 0;
                        self.handle_event(cx, line, line_ref, byte, Affinity::Before, y, column);
                        for wrapped in line_ref.wrappeds() {
                            match wrapped {
                                Wrapped::Text {
                                    is_inlay: false,
                                    text,
                                } => {
                                    for grapheme in text.graphemes() {
                                        self.handle_event(
                                            cx,
                                            line,
                                            line_ref,
                                            byte,
                                            Affinity::After,
                                            y,
                                            column,
                                        );
                                        byte += grapheme.len();
                                        column += grapheme
                                            .chars()
                                            .map(|char| {
                                                char.column_count(
                                                    session.settings().tab_column_count,
                                                )
                                            })
                                            .sum::<usize>();
                                        self.handle_event(
                                            cx,
                                            line,
                                            line_ref,
                                            byte,
                                            Affinity::Before,
                                            y,
                                            column,
                                        );
                                    }
                                }
                                Wrapped::Text {
                                    is_inlay: true,
                                    text,
                                } => {
                                    column += text
                                        .chars()
                                        .map(|char| {
                                            char.column_count(session.settings().tab_column_count)
                                        })
                                        .sum::<usize>();
                                }
                                Wrapped::Widget(widget) => {
                                    column += widget.column_count;
                                }
                                Wrapped::Wrap => {
                                    if self.active_selection.is_some() {
                                        self.draw_selection(cx, line_ref, y, column);
                                    }
                                    column = line_ref.wrap_indent_column_count();
                                    y += line_ref.scale();
                                }
                            }
                        }
                        self.handle_event(cx, line, line_ref, byte, Affinity::After, y, column);
                        column += 1;
                        if self.active_selection.is_some() {
                            self.draw_selection(cx, line_ref, y, column);
                        }
                        line += 1;
                        y += line_ref.scale();
                    }
                    Block::Line {
                        is_inlay: true,
                        line: line_ref,
                    } => {
                        y += line_ref.height();
                    }
                    Block::Widget(widget) => {
                        y += widget.height;
                    }
                }
            }
        });
        if self.active_selection.is_some() {
            self.code_editor.draw_selection.end(cx);
        }
    }

    fn handle_event(
        &mut self,
        cx: &mut Cx2d<'_>,
        line: usize,
        line_ref: Line<'_>,
        byte: usize,
        affinity: Affinity,
        y: f64,
        column: usize,
    ) {
        let point = Point { line, byte };
        if self.active_selection.as_ref().map_or(false, |selection| {
            selection.selection.end() == point && selection.selection.end_affinity() == affinity
        }) {
            self.draw_selection(cx, line_ref, y, column);
            self.code_editor.draw_selection.end(cx);
            let selection = self.active_selection.take().unwrap().selection;
            if selection.cursor == point && selection.affinity == affinity {
                self.draw_cursor(cx, line_ref, y, column);
            }
        }
        if self
            .selections
            .as_slice()
            .first()
            .map_or(false, |selection| {
                selection.start() == point && selection.start_affinity() == affinity
            })
        {
            let selection = *self.selections.next().unwrap();
            if selection.cursor == point && selection.affinity == affinity {
                self.draw_cursor(cx, line_ref, y, column);
            }
            if !selection.is_empty() {
                self.active_selection = Some(ActiveSelection {
                    selection,
                    start_x: line_ref.column_to_x(column),
                });
            }
            self.code_editor.draw_selection.begin();
        }
    }

    fn draw_selection(&mut self, cx: &mut Cx2d<'_>, line: Line<'_>, y: f64, column: usize) {
        let start_x = mem::take(&mut self.active_selection.as_mut().unwrap().start_x);
        self.code_editor.draw_selection.draw(
            cx,
            Rect {
                pos: DVec2 { x: start_x, y } * self.code_editor.cell_size
                    - self.code_editor.viewport_rect.pos,
                size: DVec2 {
                    x: line.column_to_x(column) - start_x,
                    y: line.scale(),
                } * self.code_editor.cell_size,
            },
        );
    }

    fn draw_cursor(&mut self, cx: &mut Cx2d<'_>, line: Line<'_>, y: f64, column: usize) {
        self.code_editor.draw_cursor.draw_abs(
            cx,
            Rect {
                pos: DVec2 {
                    x: line.column_to_x(column),
                    y,
                } * self.code_editor.cell_size
                    - self.code_editor.viewport_rect.pos,
                size: DVec2 {
                    x: 2.0,
                    y: line.scale() * self.code_editor.cell_size.y,
                },
            },
        );
    }
}

struct ActiveSelection {
    selection: Selection,
    start_x: f64,
}

#[derive(Live, LiveHook)]
#[repr(C)]
struct DrawSelection {
    #[deref]
    draw_super: DrawQuad,
    #[live]
    prev_x: f32,
    #[live]
    prev_w: f32,
    #[live]
    next_x: f32,
    #[live]
    next_w: f32,
    #[rust]
    prev_prev_rect: Option<Rect>,
    #[rust]
    prev_rect: Option<Rect>,
}

impl DrawSelection {
    fn begin(&mut self) {
        debug_assert!(self.prev_rect.is_none());
    }

    fn end(&mut self, cx: &mut Cx2d<'_>) {
        self.draw_rect_internal(cx, None);
        self.prev_prev_rect = None;
        self.prev_rect = None;
    }

    fn draw(&mut self, cx: &mut Cx2d<'_>, rect: Rect) {
        self.draw_rect_internal(cx, Some(rect));
        self.prev_prev_rect = self.prev_rect;
        self.prev_rect = Some(rect);
    }

    fn draw_rect_internal(&mut self, cx: &mut Cx2d, rect: Option<Rect>) {
        if let Some(prev_rect) = self.prev_rect {
            if let Some(prev_prev_rect) = self.prev_prev_rect {
                self.prev_x = prev_prev_rect.pos.x as f32;
                self.prev_w = prev_prev_rect.size.x as f32;
            } else {
                self.prev_x = 0.0;
                self.prev_w = 0.0;
            }
            if let Some(rect) = rect {
                self.next_x = rect.pos.x as f32;
                self.next_w = rect.size.x as f32;
            } else {
                self.next_x = 0.0;
                self.next_w = 0.0;
            }
            self.draw_abs(cx, prev_rect);
        }
    }
}