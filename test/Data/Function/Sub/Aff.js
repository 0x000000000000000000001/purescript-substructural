// Test-owned resources instrument actual Effect execution. No library primitive
// receives a reusable resource through a Shared boundary.
let trace = [];
let resources = [];

export const reset = () => {
  trace = [];
  resources = [];
};

export const acquire = name => value => () => {
  const resource = { name, value, id: resources.length + 1, consumed: false };
  resources.push(resource);
  trace.push(`acquire:${name}:${resource.id}`);
  return resource;
};

export const consume = resource => () => {
  if (resource.consumed) throw new Error("Resource consumed twice");
  resource.consumed = true;
  trace.push(`consume:${resource.name}:${resource.id}`);
  return resource.value;
};

export const record = event => () => { trace.push(event); };
export const events = () => trace.slice();
export const assertAllConsumed = () => {
  if (!resources.every(resource => resource.consumed)) {
    throw new Error("Owned test resource was not consumed");
  }
};
