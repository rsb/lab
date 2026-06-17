//! Exercises the `lab-core` error contract through its public surface, the way
//! a dependent crate would consume it.

use std::error::Error;
use std::fmt;

use lab_core::{Chain, Fail, Rendered};

#[derive(Debug)]
struct Leaf;

impl fmt::Display for Leaf {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("config.load failed: file is missing")
  }
}

impl Error for Leaf {}
impl Fail for Leaf {}

#[derive(Debug)]
struct Wrap {
  message: &'static str,
  source: Box<dyn Error + 'static>,
}

impl fmt::Display for Wrap {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(self.message)
  }
}

impl Error for Wrap {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    Some(self.source.as_ref())
  }
}

impl Fail for Wrap {}

#[test]
fn a_lab_error_is_an_ordinary_std_error() {
  // The supertrait commitment: a Fail value drops into the std ecosystem with
  // no conversion.
  let boxed: Box<dyn Error> = Box::new(Leaf);
  assert_eq!(boxed.to_string(), "config.load failed: file is missing");
}

#[test]
fn the_chain_is_walkable_outermost_first() {
  let err = Wrap {
    message: "session.open failed",
    source: Box::new(Leaf),
  };
  let links: Vec<String> = err.chain().map(|link| link.to_string()).collect();
  assert_eq!(
    links,
    ["session.open failed", "config.load failed: file is missing",]
  );
}

#[test]
fn the_rendering_is_the_go_style_reading() {
  let err = Wrap {
    message: "session.open failed",
    source: Box::new(Leaf),
  };
  assert_eq!(
    err.rendered().to_string(),
    "session.open failed: config.load failed: file is missing"
  );
}

#[test]
fn adapters_construct_over_any_std_error() {
  let err = Wrap {
    message: "session.open failed",
    source: Box::new(Leaf),
  };
  let as_dyn: &(dyn Error + 'static) = &err;
  assert_eq!(Chain::new(as_dyn).count(), 2);
  assert_eq!(
    Rendered::new(as_dyn).to_string(),
    "session.open failed: config.load failed: file is missing"
  );
}

#[test]
fn a_leaf_error_has_no_source() {
  // A leaf genuinely has no cause and says so: source() is None, never a stub
  // that lies about the chain's depth.
  assert!(Error::source(&Leaf).is_none());
}

#[test]
fn a_single_link_chain_renders_without_a_separator() {
  // The chain of a sourceless error is exactly one link, and Rendered adds no
  // ": " — the separator only appears between links.
  assert_eq!(Leaf.chain().count(), 1);
  assert_eq!(
    Leaf.rendered().to_string(),
    "config.load failed: file is missing"
  );
}
