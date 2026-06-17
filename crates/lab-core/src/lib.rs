//! The Lab error contract.
//!
//! `lab-core` is the root of the Lab workspace: it depends on nothing in the
//! workspace, and everything in the workspace may depend on it. The contract it
//! publishes is the most general articulation in the ecosystem, so it lives at
//! the crate root rather than inside a namespace.
//!
//! The contract is a trait, not a concrete type. `lab-core` publishes the
//! obligations; each crate that can fail defines its own error type that meets
//! them. There is no foundational error struct and no foundational error enum.
//!
//! - [`Fail`] — the contract trait. Implementing it is the opt-in act of
//!   accepting its obligations: an honest
//!   [`source()`](std::error::Error::source) chain and a grammar-conformant
//!   [`Display`](std::fmt::Display).
//! - [`Chain`] — walks an error's `source()` links, outermost first.
//! - [`Rendered`] — a [`Display`](std::fmt::Display) view over that chain,
//!   joining each link with `": "` to read `outer: middle: leaf`.
//!
//! The interoperable chain and its Go-style rendering are one causal chain seen
//! two ways: the wider Rust ecosystem reads it through `source()` like any
//! error, and Lab consumers additionally get the `": "`-joined reading for free.
//!
//! Decisions behind this shape are recorded in the "errors are values", "the
//! error contract is a trait", and "error message grammar" ADRs at
//! <https://adrs.rsb.sh>.

use std::error::Error;
use std::fmt;

/// The Lab error contract.
///
/// Every Lab error is its own crate's concrete type implementing this trait;
/// there is no shared concrete error type. Implementing `Fail` is the opt-in
/// act of accepting the contract's obligations:
///
/// - **`std::error::Error` is a supertrait**, so every Lab error is, with no
///   conversion, an ordinary member of the Rust error ecosystem — it can be
///   `?`-converted, boxed as `Box<dyn Error>`, and composed alongside errors
///   from crates that have never heard of Lab.
/// - **The causal chain is the populated [`Error::source`].** A wrapping error
///   returns its cause; a leaf returns `None` because it genuinely has none.
///   `source()` is the real chain, never a stub that lies about its own depth.
/// - **[`Display`](fmt::Display) renders this error's own level only**, phrased
///   per the message grammar (`receiver.method failed: msg`). The
///   chain-spanning `outer: middle: leaf` reading is produced by [`Rendered`]
///   walking `source()` — not by any single level restating the levels below
///   it. Because [`Rendered`] reproduces each level's message verbatim into
///   logs and UI, that message names the operation and must never carry a value
///   that is unsafe to log — a secret, a credential, or personal data.
///
/// There is deliberately **no blanket implementation** of `Fail`: a blanket
/// impl for every [`Error`] would make the contract assert nothing. A type
/// joins the contract by implementing it explicitly.
///
/// # Example
///
/// ```
/// use std::error::Error;
/// use std::fmt;
///
/// use lab_core::Fail;
///
/// #[derive(Debug)]
/// struct LoadFailed;
///
/// impl fmt::Display for LoadFailed {
///   fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
///     f.write_str("config.load failed: file is missing")
///   }
/// }
///
/// impl Error for LoadFailed {}
/// impl Fail for LoadFailed {}
///
/// #[derive(Debug)]
/// struct OpenFailed {
///   source: LoadFailed,
/// }
///
/// impl fmt::Display for OpenFailed {
///   // This level only — name the operation, not the cause beneath it.
///   fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
///     f.write_str("session.open failed")
///   }
/// }
///
/// impl Error for OpenFailed {
///   fn source(&self) -> Option<&(dyn Error + 'static)> {
///     Some(&self.source) // hold the cause honestly
///   }
/// }
///
/// impl Fail for OpenFailed {}
///
/// let err = OpenFailed { source: LoadFailed };
/// assert_eq!(
///   err.rendered().to_string(),
///   "session.open failed: config.load failed: file is missing",
/// );
/// assert_eq!(err.chain().count(), 2);
/// ```
pub trait Fail: Error {
  /// Walk this error's causal chain, outermost (this error) first.
  fn chain(&self) -> Chain<'_>
  where
    Self: Sized + 'static,
  {
    Chain::new(self)
  }

  /// Render this error's chain in the Go-style `outer: middle: leaf` reading.
  fn rendered(&self) -> Rendered<'_>
  where
    Self: Sized + 'static,
  {
    Rendered::new(self)
  }
}

/// The maximum number of links [`Chain`] yields, and the depth past which
/// [`Rendered`] truncates.
///
/// A causal chain is shallow in practice; this bound exists only so that a
/// cyclic or unbounded `source()` implementation cannot make the walk run
/// forever or allocate without end. `source()` is supposed to be acyclic and
/// finite — but the contract is implemented by other crates, including external
/// ones, so the walk defends itself rather than trusting every implementor.
/// Reaching this depth indicates a malformed chain, not ordinary use.
pub const MAX_DEPTH: usize = 128;

/// An iterator over an error's causal chain, walking
/// [`Error::source`] links outermost first.
///
/// The first item is the error the chain was created from; each subsequent item
/// is the previous item's `source()`. Iteration ends when a link reports no
/// source — or, as a safeguard against a cyclic or unbounded `source()`, after
/// [`MAX_DEPTH`] links, whichever comes first.
#[derive(Clone)]
pub struct Chain<'a> {
  current: Option<&'a (dyn Error + 'static)>,
  remaining: usize,
}

impl<'a> Chain<'a> {
  /// Begin a chain at `head`. `head` is the outermost link.
  pub fn new(head: &'a (dyn Error + 'static)) -> Self {
    Self {
      current: Some(head),
      remaining: MAX_DEPTH,
    }
  }
}

impl<'a> Iterator for Chain<'a> {
  type Item = &'a (dyn Error + 'static);

  fn next(&mut self) -> Option<Self::Item> {
    if self.remaining == 0 {
      // The cap is reached: stop rather than follow a cyclic or unbounded
      // source() forever. Clearing `current` keeps the iterator fused.
      self.current = None;
      return None;
    }
    let current = self.current?;
    self.remaining -= 1;
    self.current = current.source();
    Some(current)
  }
}

impl std::iter::FusedIterator for Chain<'_> {}

/// A [`Display`](fmt::Display) view over an error's causal chain, joining each
/// link's own `Display` with `": "` to read `outer: middle: leaf`.
///
/// It is a view, not a parallel data structure: it walks the same
/// [`Error::source`] chain that [`Chain`] does. Each link contributes its own
/// level's message only — the joining is what produces the chained reading, so
/// a link must not restate the levels beneath it.
///
/// The walk is bounded by [`MAX_DEPTH`]; if the chain is longer (a cyclic or
/// unbounded `source()`), the rendering ends with a `: …` marker so the
/// truncation is visible rather than silent.
#[derive(Clone, Copy)]
pub struct Rendered<'a> {
  head: &'a (dyn Error + 'static),
}

impl<'a> Rendered<'a> {
  /// Render the chain beginning at `head`.
  pub fn new(head: &'a (dyn Error + 'static)) -> Self {
    Self { head }
  }
}

impl fmt::Display for Rendered<'_> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let mut last: Option<&(dyn Error + 'static)> = None;
    let mut count = 0;
    for (i, link) in Chain::new(self.head).enumerate() {
      if i > 0 {
        f.write_str(": ")?;
      }
      fmt::Display::fmt(link, f)?;
      last = Some(link);
      count = i + 1;
    }
    // If the walk stopped at the cap and a further cause remains, mark the
    // truncation rather than silently dropping the tail.
    if count == MAX_DEPTH && last.is_some_and(|link| link.source().is_some()) {
      f.write_str(": …")?;
    }
    Ok(())
  }
}
