/* -------------------------------------------------------------------------- */

export const composeFFI = function(f) {
  return function(g) {
    return function(x) {
      return f(g(x));
    };
  };
};

export const idFFI = function(a) {
  return a;
};

export const runSharedFFI = function(func) {
  return function(value) {
    return func(value);
  };
};

export const liftSharedFFI = function(func) {
  return function(value) {
    return func(value);
  };
};

/* -------------------------------------------------------------------------- */

export const unsafeCloneFFI = function(Tuple) {
  return function(a) {
    return Tuple(a)(a);
  };
};

export const unsafeDrop = function(a) {
  return null;
};

export const fstFFI = function(drop) {
  return function(fst) {
    return function(snd) {
      return function(tuple) {
        drop(snd(tuple));
        return fst(tuple);
      };
    };
  };
};

export const sndFFI = function(drop) {
  return function(fst) {
    return function(snd) {
      return function(tuple) {
        drop(fst(tuple));
        return snd(tuple);
      };
    };
  };
};

/* -------------------------------------------------------------------------- */

export const cloneTupleFFI = function(Tuple) {
  return function(fst) {
    return function(snd) {
      return function(cloneA) {
        return function(cloneB) {
          return function(tuple) {
            var aClones = cloneA(fst(tuple));
            var bClones = cloneB(snd(tuple));
            return Tuple(
              Tuple(fst(aClones))(fst(bClones))
            )(
              Tuple(snd(aClones))(snd(bClones))
            );
          };
        };
      };
    };
  };
};

export const dropTupleFFI = function(fst) {
  return function(snd) {
    return function(dropA) {
      return function(dropB) {
        return function(tuple) {
          dropA(fst(tuple));
          dropB(snd(tuple));
        };
      };
    };
  };
};

/* -------------------------------------------------------------------------- */

export const borrowFFI = function(Tuple) {
  return function(func) {
    return function(value) {
      var result = func(value);
      return Tuple(value)(result);
    };
  };
};
