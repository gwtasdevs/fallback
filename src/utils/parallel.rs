#[macro_export]
macro_rules! parallel {
  ($a:expr $(,)?) => {
    match ($a)() {
      Ok(a) => Ok(a),
      a => {
        let mut errors = vec![];
        if let Err(err) = a {
          errors.push(err);
        }
        Err($crate::utils::MultiError::new(errors))
      }
    }
  };
  ($a:expr, $b:expr $(,)?) => {
    match ::rayon::join($a, $b) {
      (Ok(a), Ok(b)) => Ok((a, b)),
      (a, b) => {
        let mut errors = vec![];
        if let Err(err) = a {
          errors.push(err);
        }
        if let Err(err) = b {
          errors.push(err);
        }
        Err($crate::utils::MultiError::new(errors))
      }
    }
  };
  ($a:expr, $b:expr, $c:expr $(,)?) => {
    match ::rayon::join($a, || ::rayon::join($b, $c)) {
      (Ok(a), (Ok(b), Ok(c))) => Ok((a, b, c)),
      (a, (b, c)) => {
        let mut errors = vec![];
        if let Err(err) = a {
          errors.push(err);
        }
        if let Err(err) = b {
          errors.push(err);
        }
        if let Err(err) = c {
          errors.push(err);
        }
        Err($crate::utils::MultiError::new(errors))
      }
    }
  };
  ($a:expr, $b:expr, $c:expr, $d:expr $(,)?) => {
    match ::rayon::join(|| ::rayon::join($a, $b), || ::rayon::join($c, $d)) {
      ((Ok(a), Ok(b)), (Ok(c), Ok(d))) => Ok((a, b, c, d)),
      ((a, b), (c, d)) => {
        let mut errors = vec![];
        if let Err(err) = a {
          errors.push(err);
        }
        if let Err(err) = b {
          errors.push(err);
        }
        if let Err(err) = c {
          errors.push(err);
        }
        if let Err(err) = d {
          errors.push(err);
        }
        Err($crate::utils::MultiError::new(errors))
      }
    }
  };
  ($a:expr, $b:expr, $c:expr, $d:expr, $e:expr $(,)?) => {
    match ::rayon::join(|| ::rayon::join($a, $b), || ::rayon::join($c, || ::rayon::join($d, $e))) {
      ((Ok(a), Ok(b)), (Ok(c), (Ok(d), Ok(e)))) => Ok((a, b, c, d, e)),
      ((a, b), (c, (d, e))) => {
        let mut errors = vec![];
        if let Err(err) = a {
          errors.push(err);
        }
        if let Err(err) = b {
          errors.push(err);
        }
        if let Err(err) = c {
          errors.push(err);
        }
        if let Err(err) = d {
          errors.push(err);
        }
        if let Err(err) = e {
          errors.push(err);
        }
        Err($crate::utils::MultiError::new(errors))
      }
    }
  };
  ($a:expr, $b:expr, $c:expr, $d:expr, $e:expr, $f:expr $(,)?) => {
    match ::rayon::join(
      || ::rayon::join($a, || ::rayon::join($b, $c)),
      || ::rayon::join($d, || ::rayon::join($e, $f)),
    ) {
      ((Ok(a), (Ok(b), Ok(c))), (Ok(d), (Ok(e), Ok(f)))) => Ok((a, b, c, d, e, f)),
      ((a, (b, c)), (d, (e, f))) => {
        let mut errors = vec![];
        if let Err(err) = a {
          errors.push(err);
        }
        if let Err(err) = b {
          errors.push(err);
        }
        if let Err(err) = c {
          errors.push(err);
        }
        if let Err(err) = d {
          errors.push(err);
        }
        if let Err(err) = e {
          errors.push(err);
        }
        if let Err(err) = f {
          errors.push(err);
        }
        Err($crate::utils::MultiError::new(errors))
      }
    }
  };
  ($a:expr, $b:expr, $c:expr, $d:expr, $e:expr, $f:expr, $g:expr $(,)?) => {
    match ::rayon::join(
      || ::rayon::join($a, || ::rayon::join($b, $c)),
      || ::rayon::join(|| ::rayon::join($d, $e), || ::rayon::join($f, $g)),
    ) {
      ((Ok(a), (Ok(b), Ok(c))), ((Ok(d), Ok(e)), (Ok(f), Ok(g)))) => Ok((a, b, c, d, e, f, g)),
      ((a, (b, c)), ((d, e), (f, g))) => {
        let mut errors = vec![];
        if let Err(err) = a {
          errors.push(err);
        }
        if let Err(err) = b {
          errors.push(err);
        }
        if let Err(err) = c {
          errors.push(err);
        }
        if let Err(err) = d {
          errors.push(err);
        }
        if let Err(err) = e {
          errors.push(err);
        }
        if let Err(err) = f {
          errors.push(err);
        }
        if let Err(err) = g {
          errors.push(err);
        }
        Err($crate::utils::MultiError::new(errors))
      }
    }
  };
  ($a:expr, $b:expr, $c:expr, $d:expr, $e:expr, $f:expr, $g:expr, $h:expr $(,)?) => {
    match ::rayon::join(
      || ::rayon::join(|| ::rayon::join($a, $b), || ::rayon::join($c, $d)),
      || ::rayon::join(|| ::rayon::join($e, $f), || ::rayon::join($g, $h)),
    ) {
      (((Ok(a), Ok(b)), (Ok(c), Ok(d))), ((Ok(e), Ok(f)), (Ok(g), Ok(h)))) => Ok((a, b, c, d, e, f, g, h)),
      (((a, b), (c, d)), ((e, f), (g, h))) => {
        let mut errors = vec![];
        if let Err(err) = a {
          errors.push(err);
        }
        if let Err(err) = b {
          errors.push(err);
        }
        if let Err(err) = c {
          errors.push(err);
        }
        if let Err(err) = d {
          errors.push(err);
        }
        if let Err(err) = e {
          errors.push(err);
        }
        if let Err(err) = f {
          errors.push(err);
        }
        if let Err(err) = g {
          errors.push(err);
        }
        if let Err(err) = h {
          errors.push(err);
        }
        Err($crate::utils::MultiError::new(errors))
      }
    }
  };
}
