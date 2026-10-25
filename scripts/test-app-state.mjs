import assert from "node:assert/strict";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { build } from "esbuild";

const temporary = await mkdtemp(join(tmpdir(), "localscribe-app-test-"));
const modules = {
  react: `
    export const useState = (...args) => globalThis.appTest.useState(...args);
    export const useRef = (...args) => globalThis.appTest.useRef(...args);
    export const useMemo = (factory) => factory();
    export const useEffect = (...args) => globalThis.appTest.useEffect(...args);`,
  "react/jsx-runtime": `
    export const jsx = (type, props) => ({ type, props });
    export const jsxs = jsx;
    export const Fragment = Symbol("Fragment");`,
  "@tauri-apps/api/event": `export const listen = (...args) => globalThis.appTest.listen(...args);`,
  "@tauri-apps/api/window": `export const getCurrentWindow = () => ({ onDragDropEvent: callback => globalThis.appTest.listen("drag-drop", callback) });`,
  "@tauri-apps/plugin-dialog": `
    export const open = async () => "/tmp/input.wav";
    export const save = async () => "/tmp/transcript.md";`,
  "./lib/tauri": `
    export const listModels = async () => {
      globalThis.appTest.modelLoads++;
      return globalThis.appTest.models;
    };
    export const listSpeakerModels = async () => [];
    export const claimIntegrationRequest = async () => {
      if (globalThis.appTest.claim) return globalThis.appTest.claim();
      const request = globalThis.appTest.integrationRequest;
      globalThis.appTest.integrationRequest = null;
      return request;
    };
    export const transcribeFile = options => globalThis.appTest.transcribe(options);
    export const exportTranscript = async request => { globalThis.appTest.exports.push(request); };
    export const errorMessage = error => String(error);
    export const cancelTranscription = async () => true;
    export const completeIntegrationRequest = async () => ({ transcriptPath: "/tmp/result.md" });
    export const failIntegrationRequest = async () => {};
    export const deleteModel = async () => [];
    export const deleteSpeakerModel = async () => [];
    export const downloadModel = async () => {};
    export const downloadSpeakerModel = async () => {};
    export const selectCustomModel = async () => {};
    export const selectCustomSpeakerModel = async () => {};`
};

try {
  const outfile = join(temporary, "app.mjs");
  await build({
    bundle: true, entryPoints: ["src/App.tsx"], format: "esm", platform: "node", jsx: "automatic", outfile,
    plugins: [{
      name: "app-boundaries",
      setup(builder) {
        builder.onResolve({ filter: /^(react|react\/jsx-runtime|@tauri-apps\/.*|\.\/lib\/tauri)$/ }, args =>
          args.path in modules ? { path: args.path, namespace: "test" } : undefined);
        builder.onLoad({ filter: /.*/, namespace: "test" }, args => ({ contents: modules[args.path], loader: "js" }));
      }
    }]
  });
  const { default: App } = await import(pathToFileURL(outfile).href);
  globalThis.window = {
    __TAURI_INTERNALS__: {},
    matchMedia: () => ({ matches: true, addEventListener() {}, removeEventListener() {} }),
    addEventListener() {}, removeEventListener() {}, setInterval: () => 1, clearInterval() {}
  };
  globalThis.localStorage = { getItem: () => null, setItem() {} };
  Object.defineProperty(globalThis, "navigator", { configurable: true, value: { language: "en" } });

  function harness() {
    const test = {
      slots: [], pendingEffects: [], pendingListeners: [], listeners: [], exports: [], modelLoads: 0,
      cursor: 0, tree: null, jobs: [],
      models: [{ id: "large-v3-turbo-q5_0", installed: true, path: "/tmp/model.bin", name: "Test model" }],
      useState(initial) {
        const index = this.cursor++;
        if (!(index in this.slots)) this.slots[index] = typeof initial === "function" ? initial() : initial;
        return [this.slots[index], value => {
          this.slots[index] = typeof value === "function" ? value(this.slots[index]) : value;
        }];
      },
      useRef(initial) {
        const index = this.cursor++;
        return this.slots[index] ??= { current: initial };
      },
      useEffect(callback, dependencies) {
        const index = this.cursor++;
        const previous = this.slots[index];
        if (previous && dependencies.every((value, i) => Object.is(value, previous.dependencies[i]))) return;
        this.pendingEffects.push(() => {
          previous?.cleanup?.();
          this.slots[index] = { dependencies, cleanup: callback() };
        });
      },
      listen(name, callback) {
        const listener = { name, callback, active: true };
        this.listeners.push(listener);
        return new Promise(resolve => this.pendingListeners.push(() => resolve(() => { listener.active = false; })));
      },
      transcribe(options) {
        this.jobs.push(options);
        this.options = options;
        return new Promise((resolve, reject) => { this.resolveJob = resolve; this.rejectJob = reject; });
      },
      render() {
        this.cursor = 0;
        this.tree = App();
        this.pendingEffects.splice(0).forEach(effect => effect());
      },
      resolveListeners() { this.pendingListeners.splice(0).forEach(resolve => resolve()); },
      unmount() { this.slots.forEach(slot => slot?.cleanup?.()); },
      emit(name, payload) {
        this.listeners.filter(listener => listener.active && listener.name === name)
          .forEach(listener => listener.callback({ payload }));
      },
      find(predicate) {
        function visit(element) {
          if (!element || typeof element !== "object") return;
          if (predicate(element)) return element;
          for (const child of [element.props?.children].flat(Infinity)) {
            const found = visit(child);
            if (found) return found;
          }
        }
        const result = visit(this.tree);
        assert(result, "expected an element in App's output");
        return result;
      },
      editor() { return this.find(element => element.props?.className === "transcriptEditor"); },
      button(label) { return this.find(element => element.type === "button" && element.props.children === label); }
    };
    globalThis.appTest = test;
    return test;
  }
  const settle = () => new Promise(setImmediate);
  async function initialized(options = {}) {
    const test = Object.assign(harness(), options);
    test.render();
    await settle();
    test.resolveListeners();
    await settle();
    test.render();
    return test;
  }
  async function start(test) {
    await test.find(element => element.type === "button" && element.props.onClick?.name === "chooseInputFile").props.onClick();
    test.render();
    test.render();
    test.resolveListeners();
    await settle();
  }

  const late = harness();
  late.render();
  late.unmount();
  late.resolveListeners();
  await settle();
  assert.equal(late.listeners.filter(listener => listener.active).length, 0, "late subscriptions must be disposed");

  const edited = await initialized();
  await start(edited);
  assert.equal(edited.options.speakerMode, "auto");
  assert.equal("speakerModelId" in edited.options, false);
  assert.equal(edited.editor().props.readOnly, true);
  edited.resolveJob({ jobId: edited.options.jobId, text: "Original words", segments: [{ index: 0, startMs: 0, endMs: 1000, text: "Original words" }] });
  await settle();
  edited.render();
  assert.equal(edited.editor().props.readOnly, false);
  edited.editor().props.onChange({ target: { value: "Edited words" } });
  edited.render();
  await edited.button("MD").props.onClick();
  assert.equal(edited.exports[0].text, "Edited words");
  assert.deepEqual(edited.exports[0].segments, [], "edited text must not export stale segments");
  edited.unmount();

  const failed = await initialized();
  await start(failed);
  failed.rejectJob("Whisper failed");
  await settle();
  failed.render();
  await settle();
  failed.render();
  assert.equal(failed.modelLoads, 1, "job changes must not reload models and clear errors");
  failed.find(element => element.props?.children === "Whisper failed");
  failed.unmount();

  const live = await initialized();
  const staleError = live.listeners.find(listener => listener.name === "transcription-error").callback;
  await start(live);
  staleError({ payload: { jobId: "old", message: "Old error" } });
  live.emit("transcription-error", { jobId: "foreign", message: "Foreign error" });
  live.render();
  assert.equal(live.editor().props.readOnly, true, "stale errors must not clear the active job");
  for (const [index, startMs] of [0, 1000, 2000, 11500].entries()) {
    live.emit("transcription-segment", { jobId: live.options.jobId, segment: { index, startMs, endMs: startMs + 500, text: "Please confirm the meeting time." } });
  }
  live.render();
  assert.equal(live.editor().props.value.split("\n").length, 2, "discarded repeats must remain in raw history");
  live.resolveJob({ jobId: live.options.jobId, text: "Finished", segments: [] });
  await settle();
  live.unmount();
  const dropped = await initialized();
  dropped.emit("drag-drop", { type: "enter", paths: ["/tmp/recording.M4A"] });
  dropped.render();
  dropped.find(element => element.props?.className?.includes("dropZone dragging"));
  dropped.emit("drag-drop", { type: "drop", paths: ["/tmp/recording.M4A"] });
  dropped.render();
  dropped.render();
  assert.equal(dropped.jobs.length, 1, "a native drop starts exactly one job without a start button");
  assert.equal(dropped.options.filePath, "/tmp/recording.M4A");
  assert.equal(dropped.options.speakerMode, "auto");
  dropped.emit("drag-drop", { type: "drop", paths: ["/tmp/other.wav"] });
  dropped.render();
  assert.equal(dropped.jobs.length, 1, "dropping while busy must not replace the active recording");
  dropped.resolveJob({ jobId: dropped.options.jobId, text: "Speaker 1: Hello", segments: [] });
  await settle();
  dropped.render();
  await settle();
  dropped.emit("drag-drop", { type: "drop", paths: ["/tmp/notes.txt"] });
  dropped.render();
  assert.equal(dropped.jobs.length, 1, "unsupported files must not start a job");
  assert.equal(dropped.editor().props.value, "Speaker 1: Hello", "invalid drops retain the result");
  dropped.find(element => element.props?.className === "errorBox");
  dropped.emit("drag-drop", { type: "drop", paths: ["/tmp/a.wav", "/tmp/b.wav"] });
  dropped.render();
  dropped.find(element => element.props?.children === "Drop one recording at a time.");
  assert.equal(dropped.jobs.length, 1);
  dropped.emit("drag-drop", { type: "drop", paths: ["/tmp/recording.M4A"] });
  dropped.render();
  assert.equal(dropped.jobs.length, 2, "the same file can be dropped again after completion");
  dropped.resolveJob({ jobId: dropped.options.jobId, text: "Again", segments: [] });
  await settle();
  dropped.unmount();

  const waiting = await initialized({ models: [] });
  waiting.emit("drag-drop", { type: "drop", paths: ["/tmp/waiting.wav"] });
  waiting.render();
  assert.equal(waiting.jobs.length, 0, "no job can start without a model");
  const manager = waiting.find(element => element.props?.onUse && element.props?.onChooseCustom);
  const installed = { id: "custom", installed: true, path: "/tmp/new-model.bin", name: "Custom" };
  // Simulate installation in the existing model manager.
  waiting.slots[0] = [installed];
  manager.props.onUse(installed);
  waiting.render();
  assert.equal(waiting.options.filePath, "/tmp/waiting.wav", "installation resumes the pending recording");
  waiting.resolveJob({ jobId: waiting.options.jobId, text: "Ready", segments: [] });
  await settle();
  waiting.unmount();

  let releaseClaim;
  const polling = await initialized({ claim: () => new Promise(resolve => { releaseClaim = resolve; }) });
  polling.emit("drag-drop", { type: "drop", paths: ["/tmp/while-polling.wav"] });
  polling.render();
  assert.equal(polling.jobs.length, 0, "a drop waits for an in-flight integration claim");
  releaseClaim(null);
  await settle();
  polling.render();
  assert.equal(polling.options.filePath, "/tmp/while-polling.wav", "polling must not silently discard a dropped recording");
  polling.resolveJob({ jobId: polling.options.jobId, text: "Queued", segments: [] });
  await settle();
  polling.unmount();

  const integrated = await initialized({ integrationRequest: { id: "meeting", mediaPath: "/tmp/meeting.webm", options: { speakerMode: "off" } } });
  assert.equal(integrated.options.speakerMode, "auto", "legacy integration options cannot disable speakers");
  integrated.resolveJob({ jobId: "meeting", text: "Meeting", segments: [] });
  await settle();
  integrated.unmount();
  console.log("App regressions passed: automatic import, validation, busy drops, model setup, speakers, edits and events.");
} finally {
  await rm(temporary, { recursive: true, force: true });
}
