import assert from "node:assert/strict";
import { readFileSync, existsSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import vm from "node:vm";

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const src = join(root, "src");
const read = (path) => readFileSync(join(root, path), "utf8");
const main = read("src/main.js");
const index = read("src/index.html");
const cancel = read("src/cancel.js");
const cancelHtml = read("src/cancel.html");
const overlay = read("src/overlay.js");
const overlayHtml = read("src/overlay.html");
const rust = read("src-tauri/src/main.rs");
const errors = [];
const check = (name, fn) => {
  try { fn(); }
  catch (error) { errors.push(`${name}: ${error.message}`); }
};

check("main DOM references", () => {
  for (const [script, html] of [[main, index], [cancel, cancelHtml], [overlay, overlayHtml]]) {
    const ids = new Set([...html.matchAll(/\bid="([^"]+)"/g)].map((match) => match[1]));
    const references = [...script.matchAll(/(?:\bbyId|\bgetElementById)\("([^"]+)"\)/g)].map((match) => match[1]);
    assert.deepEqual([...new Set(references.filter((id) => !ids.has(id)))], []);
  }
});

check("invoke commands", () => {
  const handler = rust.match(/tauri::generate_handler!\[([\s\S]*?)\]\)/)?.[1] || "";
  const registered = new Set(handler.match(/[a-z][a-z0-9_]*/g) || []);
  const commands = [...(main + cancel + overlay).matchAll(/\binvoke(?:WhenReady)?\("([a-z0-9_]+)"/g)].map((match) => match[1]);
  assert.deepEqual([...new Set(commands.filter((command) => !registered.has(command)))], []);
});

check("frontend event references", () => {
  const frontend = main + cancel + overlay;
  const listened = [...frontend.matchAll(/\blisten\("([^"]+)"/g)].map((match) => match[1]);
  const emitted = rust + frontend;
  assert.deepEqual([...new Set(listened.filter((event) => !emitted.includes(`"${event}"`)))], []);
});

check("translation references", () => {
  const dictionaryEnd = main.indexOf("const fields =");
  const dictionarySource = `${main.slice(0, dictionaryEnd)}\nglobalThis.__dictionaries = dictionaries;`;
  const sandbox = {
    window: { __TAURI__: { core: {}, event: {}, dialog: {}, window: { getCurrentWindow() {} } } },
    navigator: { platform: "MacIntel", language: "en" },
    document: { getElementById() {}, documentElement: { dataset: {} } },
  };
  vm.runInNewContext(dictionarySource, sandbox);
  const dictionaries = sandbox.__dictionaries;
  assert.deepEqual(Object.keys(dictionaries["zh-CN"]).sort(), Object.keys(dictionaries.en).sort());
  const usage = main.slice(dictionaryEnd) + index;
  const dynamic = new Set(["theme_light", "theme_dark", "switch_theme_light", "switch_theme_dark"]);
  const unused = Object.keys(dictionaries.en).filter((key) => !dynamic.has(key) && !usage.includes(`"${key}"`));
  assert.deepEqual(unused, []);
  for (const key of [...index.matchAll(/data-i18n(?:-placeholder|-aria)?="([^"]+)"/g)].map((match) => match[1])) {
    assert.ok(Object.hasOwn(dictionaries["zh-CN"], key), `missing zh-CN key ${key}`);
    assert.ok(Object.hasOwn(dictionaries.en, key), `missing en key ${key}`);
  }
});

check("capability scope", () => {
  const capabilities = readdirSync(join(root, "src-tauri/capabilities"))
    .filter((file) => file.endsWith(".json"))
    .flatMap((file) => {
      const capability = JSON.parse(read(`src-tauri/capabilities/${file}`));
      return Array.isArray(capability) ? capability : [capability];
    });
  const mainCapability = capabilities.find((capability) => capability.identifier === "main-capability");
  const overlayCapability = capabilities.find((capability) => capability.identifier === "overlay-capability");
  assert.deepEqual(mainCapability?.windows, ["main"]);
  assert.deepEqual(overlayCapability?.windows, ["overlay", "overlay-cancel"]);
  assert.deepEqual(overlayCapability?.permissions, ["core:event:allow-listen"]);
});

check("known dead files and duplicate binding", () => {
  assert.equal(existsSync(join(src, "assets/home-waveform.png")), false);
  assert.equal(existsSync(join(root, "src-tauri/icons/icon.png")), false);
  assert.equal((main.match(/byId\("refresh-models"\)\.addEventListener/g) || []).length, 1);
});

check("fixed regressions", () => {
  assert.match(main, /Promise\.allSettled\(/);
  assert.match(main, /new Option\(config\.input_device, config\.input_device\)/);
  assert.match(main, /configSaveQueue = configSaveQueue\.catch\(/);
  assert.match(main, /schema_version: 2/);
  assert.match(main, /shell\.inert = true/);
  assert.match(main, /state\.textPolish\?\.model\?\.state === "update_available"/);
  assert.match(cancel, /listen\("ui-language-changed"/);
  assert.match(index, /id="autostart-error"/);
  assert.doesNotMatch(main, /listen\("text-polish-changed"/);
});

if (errors.length) {
  console.error(errors.map((error) => `- ${error}`).join("\n"));
  process.exitCode = 1;
} else {
  console.log("Static checks passed.");
}
