// Instrumentation belongs to the trusted test FFI, not the public library.
const drops = [];

export const trackedDrop = resource => {
  if (resource.dropped) throw new Error("Tracked resource destroyed twice");
  resource.dropped = true;
  drops.push(resource.name);
};

export const assertDrops = dropPair => keepSecond => keepFirst => Tuple => () => {
  drops.length = 0;
  dropPair(Tuple({ name: "left" })({ name: "right" }));
  if (drops.join(",") !== "left,right") {
    throw new Error("Tuple Drop must destroy each component once, in order");
  }
  const second = keepSecond(Tuple({ name: "discard-first" })(17));
  const first = keepFirst(Tuple(23)({ name: "discard-second" }));
  if (second !== 17 || first !== 23) {
    throw new Error("Projection must preserve the retained component");
  }
  if (drops.join(",") !== "left,right,discard-first,discard-second") {
    throw new Error("Projection must execute the discarded component's Drop");
  }
};
