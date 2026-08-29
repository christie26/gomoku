use crate::{Gomoku, Pattern, Position, Stone};

#[derive(Debug)]
pub(crate) struct LineScan {
    contig_my: i32,
    end_open: bool,
    total_my: i32,
    empty_count: i32,
    hole: bool,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct PlayerPatterns {
    pub(crate) open_two: Vec<Pattern>,
    pub(crate) open_three: Vec<Pattern>,
    pub(crate) free_three: Vec<Pattern>,
    pub(crate) open_four: Vec<Pattern>,
    pub(crate) block_four: Vec<Pattern>,
    pub(crate) five_row: Vec<Pattern>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum PatternKind {
    OpenTwo,
    OpenThree,
    FreeThree,
    BlockFour,
    OpenFour,
    FiveRow,
}

pub(crate) fn classify(
    plus: &LineScan,
    minus: &LineScan,
    center_stone: i32,
) -> Option<PatternKind> {
    let total = plus.total_my + minus.total_my + center_stone;
    let contig_total = if center_stone != 0 { plus.contig_my + minus.contig_my + center_stone } else {plus.contig_my.max(minus.contig_my)};
    let empty = plus.empty_count + minus.empty_count + (1 - center_stone);
    if contig_total == 5 && center_stone != 0 {
        Some(PatternKind::FiveRow)
    } else if contig_total == 4 && plus.empty_count > 0 && minus.empty_count > 0 {
        Some(PatternKind::OpenFour)
    } else if contig_total == 4 && (plus.end_open || minus.end_open) {
        Some(PatternKind::BlockFour)
    } else if total == 4 && empty == 1 {
        Some(PatternKind::BlockFour)
    } else if total == 3 && empty == 3 {
        Some(PatternKind::FreeThree)
    } else if total == 3 && empty == 2 {
        Some(PatternKind::OpenThree)
    } else if total == 2 && plus.empty_count > 0 && minus.empty_count > 0 {
        Some(PatternKind::OpenTwo)
    } else {
        None
    }
}

pub(crate) fn build_pattern_range(
    kind: PatternKind,
    dx: i32,
    dy: i32,
    x0: i32,
    y0: i32,
    plus: &LineScan,
    minus: &LineScan,
    center_stone: i32,
) -> Pattern {
    let (lower, upper) = if kind == PatternKind::FiveRow {
        (-minus.contig_my, plus.contig_my)
    } else if kind == PatternKind::BlockFour {
        let contig_total = if center_stone != 0 { plus.contig_my + minus.contig_my + center_stone } else {plus.contig_my.max(minus.contig_my)};
        if contig_total == 4 {
          if plus.empty_count > 0 {
              (
                  -(minus.total_my + minus.empty_count + 1),
                  plus.total_my + plus.empty_count,
              )
          } else if minus.empty_count > 0 {
              (
                  -(minus.total_my + minus.empty_count),
                  plus.total_my + plus.empty_count + 1,
              )
          } else {
              (
                  -(minus.total_my + minus.empty_count),
                  plus.total_my + plus.empty_count,
              )
          }
        } else {
              (
                  -(minus.total_my + minus.empty_count),
                  plus.total_my + plus.empty_count,
              )
        }
    }
    else {
        (
            -(minus.total_my + minus.empty_count),
            plus.total_my + plus.empty_count,
        )
    };
    (lower..=upper).map(|i| (x0 + dx * i, y0 + dy * i)).collect()
}

pub(crate) fn free_three_for_move(dx: i32, dy: i32, x0: i32, y0: i32, plus: &LineScan, minus: &LineScan) -> Option<Pattern> {
    if plus.total_my + minus.total_my != 2
    || plus.empty_count + minus.empty_count < 3
    || ! plus.end_open
    || ! minus.end_open {
        return None;
    }

    // adjust empty space number because [..0.@0..] in this case, empty space on the rightest side is useless
    let mut adjusted_plus_empty = plus.empty_count;
    let mut adjusted_minus_empty = minus.empty_count;

    if plus.hole && minus.empty_count == 2 {
        adjusted_minus_empty = 1;
    }
    if minus.hole && plus.empty_count == 2 {
        adjusted_plus_empty = 1;
    }

    let plus_end = adjusted_plus_empty + plus.total_my;
    let minus_end = adjusted_minus_empty + minus.total_my;

    Some((-minus_end..=plus_end).map(|i| (x0 + dx * i, y0 + dy * i)).collect())
}

pub(crate) fn free_three_for_capture(dx: i32, dy: i32, x0: i32, y0: i32, plus: &LineScan, minus: &LineScan) -> Vec<Pattern> {
    let mut out = Vec::new();
    let plus_three = plus.total_my == 3 && plus.empty_count == 2;
    let minus_three = minus.total_my == 3 && minus.empty_count == 2;

    if plus_three || minus_three {
        if plus_three {
            out.push((0..=(plus.total_my + plus.empty_count)).map(|i| (x0 + dx * i, y0 + dy * i)).collect());
        }
        if minus_three {
            out.push((-(minus.total_my + minus.empty_count)..=0).map(|i| (x0 + dx * i, y0 + dy * i)).collect());
        }
    } else if !plus.hole && !minus.hole && plus.total_my + minus.total_my == 3 && plus.empty_count > 0 && minus.empty_count > 0 {
        let plus_end = plus.total_my + 1;
        let minus_end = minus.total_my + 1;
        out.push((-minus_end..=plus_end).map(|i| (x0 + dx * i, y0 + dy * i)).collect());
    }
    out
}

pub fn position_name(pos: &(i32, i32)) -> String {
        let (y, x) = pos;
        let x = "abcdefghijklmnopqrstuvwxyz".chars().nth(*x as usize).unwrap_or('-');
        let y = y + 1;
        format!("{x}{y}")
}

pub(crate) fn print_pattern_kind(name: &str, patterns: &[Pattern]) {
    let rendered: Vec<String> = patterns
        .iter()
        .map(|pattern| {
            let positions: Vec<String> = pattern.iter().map(position_name).collect();
            format!("[{}]", positions.join(","))
        })
        .collect();
    println!("  {name}: {} {}", patterns.len(), rendered.join(" "));
}

impl Gomoku {
    pub(crate) fn scan_line(&self, sign: i32, dx: i32, dy: i32, x0: i32, y0: i32) -> LineScan {
        self.scan_line_as(self.current_player, sign, dx, dy, x0, y0)
    }

    pub(crate) fn scan_line_as(&self, me: Stone, sign: i32, dx: i32, dy: i32, x0: i32, y0: i32) -> LineScan {
        let opponent = match me {
            Stone::Black => Stone::White,
            Stone::White => Stone::Black,
            Stone::Empty => Stone::Empty,
        };
        let mut contig_my = 0;
        let mut end_open = false;
        let mut contig_done = false;
        let mut total_my = 0;
        let mut empty_count = 0;
        let mut hole = false;
        let mut i = 1;

        loop {
            let x = x0 + dx * i * sign;
            let y = y0 + dy * i * sign;

            if !self.is_on_board(x, y)
                || self.board[x as usize][y as usize] == opponent
            {
                if empty_count == 2 {
                  end_open = true;
                }
                break;
            }
            else if empty_count == 2 {
              end_open = true;
              break;
            }

            if self.board[x as usize][y as usize] == me {
                // my stone
                if !contig_done {
                  contig_my += 1;
                } else {
                  hole = true;
                }
                total_my += 1;
            } else {
                // empty
                contig_done = true;
                empty_count += 1;
            }
            i += 1;
        }

        LineScan {
            contig_my,
            end_open,
            total_my,
            empty_count,
            hole,
        }
    }

    pub(crate) fn patterns_mut(&mut self, kind: PatternKind, player: &Stone) -> &mut Vec<Pattern> {
        let p = self.patterns.get_mut(player).unwrap();
        match kind {
            PatternKind::OpenTwo => &mut p.open_two,
            PatternKind::OpenThree => &mut p.open_three,
            PatternKind::FreeThree => &mut p.free_three,
            PatternKind::BlockFour => &mut p.block_four,
            PatternKind::OpenFour => &mut p.open_four,
            PatternKind::FiveRow => &mut p.five_row,
        }
    }

    pub(crate) fn patterns_ref(&self, kind: PatternKind, player: &Stone) -> &Vec<Pattern> {
        let p = self.patterns.get(player).unwrap();
        match kind {
            PatternKind::OpenTwo => &p.open_two,
            PatternKind::OpenThree => &p.open_three,
            PatternKind::FreeThree => &p.free_three,
            PatternKind::BlockFour => &p.block_four,
            PatternKind::OpenFour => &p.open_four,
            PatternKind::FiveRow => &p.five_row,
        }
    }

    fn rescan_pattern(&self, pattern: &Pattern, pos: Position, player: &Stone) -> Option<(PatternKind, Pattern)> {
        if pattern.len() < 2 {
            return None;
        }
        let (dx, dy) = (pattern[1].0 - pattern[0].0, pattern[1].1 - pattern[0].1);

        let &(ax, ay) = pattern.iter().find(|&&(px, py)| {
            (px, py) != pos && self.board[px as usize][py as usize] == *player
        })?;

        let plus = self.scan_line_as(*player, 1, dx, dy, ax, ay);
        let minus = self.scan_line_as(*player, -1, dx, dy, ax, ay);
        let kind = classify(&plus, &minus, 1)?;
        Some((kind, build_pattern_range(kind, dx, dy, ax, ay, &plus, &minus, 1)))
    }

    pub(crate) fn register(&mut self, kind: PatternKind, player: &Stone, pattern: Pattern) {
        let list = self.patterns_mut(kind, player);
        if !list.contains(&pattern) {
            list.push(pattern);
        }
    }
}
