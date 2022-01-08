use crate::text::*;
use std::rc::Rc;

// Regex
// -----

// Text
// ----

//TypeScript
//export type State = {
//  code: string,
//  index: number
//};
//export type Parser<A> = (state: State) => [State, A];
//Rust:
#[derive(Clone, Copy, Debug)]
pub struct State<'a> {
  pub code: &'a Text,
  pub index: usize,
}

pub type Parser<'a, A> = Rc<dyn Fn(State) -> (State, A) + 'a>;

pub fn debug(state: State) {
  let slice = &state.code[state.index..];
  let slice_str = text_to_utf8(slice);
  println!("{}", slice_str);
}

pub fn read<A>(parser: fn(state: State) -> (State, A), code: &Text) -> A {
  let (state, value) = parser(State { code, index: 0 });
  return value;
}

pub fn skip_comment(mut state: State) -> (State, bool) {
  if state.index + 1 < state.code.len() && equal_at(&state.code, &vec!['/', '/'], state.index) {
    state.index += 2;
    while state.index < state.code.len() && equal_at(&state.code, &vec!['\n'], state.index) {
      state.index += 1;
    }
    (state, true)
  } else {
    (state, false)
  }
}

pub fn skip_spaces(mut state: State) -> (State, bool) {
  if state.index < state.code.len() && equal_at(&state.code, &vec![' '], state.index) {
    state.index += 1;
    while state.index < state.code.len() && equal_at(&state.code, &vec![' '], state.index) {
      state.index += 1;
    }
    (state, true)
  } else {
    (state, false)
  }
}

pub fn skip(state: State) -> (State, bool) {
  let (state, comment) = skip_comment(state);
  let (state, spaces) = skip_spaces(state);
  if comment || spaces {
    let (state, skipped) = skip(state);
    return (state, true);
  } else {
    return (state, false);
  }
}

pub fn match_here<'a>(c: Rc<Text>) -> Parser<'a, bool> {
  return Rc::new(move |state| {
    if equal_at(&state.code, &c, state.index) {
      return (
        State {
          code: state.code,
          index: state.index + c.len(),
        },
        true,
      );
    } else {
      return (state, false);
    }
  });
}

pub fn until<'a, A: 'a>(delim: Parser<'a, bool>, parser: Parser<'a, A>) -> Parser<'a, Vec<A>> {
  Rc::new(move |state| {
    let mut ret = Vec::new();
    let mut delimited = true;
    let mut state = state;
    while delimited {
      let (new_state, new_delimited) = delim(state);
      if new_delimited {
        let (new_state, parsed) = parser(new_state);
        ret.push(parsed);
        state = new_state;
      } else {
        state = new_state;
      }
      delimited = new_delimited;
    }
    (state, ret)
  })
}

pub fn matchs<'a>(match_code: Rc<Text>) -> Parser<'a, bool> {
  return Rc::new(move |state| {
    let (state, skipped) = skip(state);
    return match_here(match_code.clone())(state);
  });
}

//fn consume(c: &'static Text) -> Parser<()> {
//  return Rc::new(move |state| {
//    let (state, matched) = match_here(c)(state);
//    if matched {
//      return (state, ());
//    } else {
//      return expected_string(c)(state);
//    }
//  });
//}

pub fn get_char<'a>() -> Parser<'a, char> {
  return Rc::new(move |state| {
    let (state, skipped) = skip(state);
    if state.index < state.code.len() {
      return (
        State {
          code: state.code,
          index: state.index + 1,
        },
        state.code[state.index],
      );
    } else {
      return (state, '\0');
    }
  });
}

pub fn done<'a>() -> Parser<'a, bool> {
  return Rc::new(move |state| {
    let (state, skipped) = skip(state);
    return (state, state.index == state.code.len());
  });
}

pub fn guard<'a, A: 'a>(head: Parser<'a, bool>, body: Parser<'a, A>) -> Parser<'a, Option<A>> {
  Rc::new(move |state| {
    let (state, skipped) = skip(state);
    let (state, matched) = dry(head.clone())(state);
    if matched {
      let (state, got) = body(state);
      (state, Some(got))
    } else {
      (state, None)
    }
  })
}

// Name
// ====

// Parses a name right after the parsing cursor.
fn name_here(state: State) -> (State, Text) {
  let mut state = state.clone();
  let mut name = Vec::<char>::new();
  let re = Regex::new(r"^[.0-9A-Z_a-z]$").unwrap();
  let state_index = state.index;
  state
    .code
    .iter()
    .skip(state.index)
    .take_while(|&&ch| re.is_match(&text_to_utf8(&[ch])))
    .for_each(|&ch| {
      name.push(ch);
      state.index += 1;
    });
  state.index += state_index;
  (state, name)
}

// Parses a name after skipping.
fn name(state: State) -> (State, Text) {
  name_here(skip(state).0)
}

// Parses a non-empty name after skipping.
fn name1(state: State) -> (State, Text) {
  let (state, name1) = name(state);
  if name1.len() > 0 {
    (state, name1)
  } else {
    lazy_static! {
      static ref N: Vec<char> = utf8_to_text("name");
    }
    expected_type(&N)(state)
  }
}

// Combinators
// ===========

pub fn grammar<'a, A: 'a>(
  name: &'a Text,
  choices: Vec<Parser<'a, Option<A>>>,
) -> Parser<'a, Option<A>> {
  Rc::new(move |state| {
    for choice in &choices {
      let (state, result) = choice(state);
      match result {
        Some(value) => {
          return (state, Some(value));
        }
        None => {}
      };
    }
    (state, None)
  })
}

pub fn dry<'a, A: 'a>(parser: Parser<'a, A>) -> Parser<'a, A> {
  Rc::new(move |state| {
    let (state, result) = parser(state);
    return (state, result);
  })
}

pub fn expected_string<A>(c: &'static Text) -> Parser<A> {
  Rc::new(move |state| {
    panic!(
      "Expected '{}':\n{}",
      "TODO_text_to_utf8", "TODO_HIGHLIGHT_FUNCTION"
    );
  })
}

pub fn expected_type<'a, A: 'a>(name: &'a Text) -> Parser<A> {
  Rc::new(move |state| {
    panic!(
      "Expected {}:\n{}",
      "TODO_text_to_utf8", "TODO_HIGHLIGHT_FUNCTION"
    );
  })
}

// Evaluates a list-like parser, with an opener, separator, and closer.
pub fn list<'a, A: 'a, B: 'a>(
  open: Parser<'a, bool>,
  sep: Parser<'a, bool>,
  close: Parser<'a, bool>,
  elem: Parser<'a, A>,
  make: fn(x: Vec<A>) -> B,
) -> Parser<'a, B> {
  Rc::new(move |state| {
    let (state, skp) = open(state);
    let (state, arr) = until(
      close.clone(),
      Rc::new(|state| {
        let (state, val) = elem(state);
        let (state, skp) = sep(state);
        (state, val)
      }),
    )(state);
    (state, make(arr))
  })
}
