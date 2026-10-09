export const assertCloneIndependence = clone => reverse => fst => snd => () => {
  const copies = clone([1, 2, 3]);
  const left = fst(copies);
  const right = snd(copies);
  if (left === right) throw new Error("UniqueArray clone must allocate separate arrays");
  const reversed = reverse(left);
  if (reversed.join(",") !== "3,2,1" || right.join(",") !== "1,2,3") {
    throw new Error("Mutating one unique clone must leave the other unchanged");
  }
  const emptyCopies = clone([]);
  if (fst(emptyCopies) === snd(emptyCopies)) {
    throw new Error("Empty unique clones must also be separate mutable arrays");
  }
};
