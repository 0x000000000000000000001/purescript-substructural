/* -------------------------------------------------------------------------- */

export const cloneUniqueArrayFFI = function(clone) {
  return function(Tuple) {
    return function(fst) {
      return function(snd) {
        return function(array) {
          var length = array.length;
          var left = Array(length);
          var right = Array(length);
          for (var i = 0; i < length; ++i) {
            var clones = clone(array[i]);
            left[i] = fst(clones);
            right[i] = snd(clones);
          }
          return Tuple(left)(right);
        };
      };
    };
  };
};

export const dropUniqueArrayFFI = function(drop) {
  return function(array) {
    var length = array.length;
    for (var i = 0; i < length; ++i) {
      drop(array[i]);
    }
  };
};

/* -------------------------------------------------------------------------- */

export const empty = function(unit) {
  return [];
};

export const singleton = function(element) {
  return [element];
};

export const fromSharedFFI = function(array) {
  return array.slice();
};

export const toSharedFFI = function(array) {
  return array;
};

/* -------------------------------------------------------------------------- */

export const snocFFI = function(fst) {
  return function(snd) {
    return function(tuple) {
      var array = fst(tuple);
      var element = snd(tuple);
      array.push(element);
      return array;
    };
  };
};

/* -------------------------------------------------------------------------- */

export const length = function(array) {
  return array.length;
};

/* -------------------------------------------------------------------------- */

export const reverse = function(array) {
  return array.reverse();
};
