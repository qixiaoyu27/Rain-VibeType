const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const button = document.getElementById("cancel");
document.documentElement.dataset.theme = localStorage.getItem("rain-theme") || "light";
window.addEventListener("storage", () => {
  document.documentElement.dataset.theme = localStorage.getItem("rain-theme") || "light";
});
listen("overlay-status", ({ payload }) => {
  document.documentElement.dataset.theme = localStorage.getItem("rain-theme") || "light";
  button.style.setProperty("--overlay-opacity", String(payload.opacity ?? 0.10));
});

function setLanguage(language) {
  const english = language === "en" || (language === "system" && !navigator.language.toLowerCase().startsWith("zh"));
  button.innerHTML = english ? "Esc&nbsp;&nbsp;Cancel" : "Esc&nbsp;&nbsp;取消";
  button.setAttribute("aria-label", english ? "Cancel recognition" : "取消识别");
}

listen("ui-language-changed", ({ payload }) => setLanguage(payload));

async function getConfigWhenReady() {
  for (let attempt = 0; attempt < 20; attempt += 1) {
    try { return await invoke("get_config"); }
    catch (error) {
      if (!String(error).includes("state not managed")) throw error;
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
  }
  throw new Error("Rain state is not ready");
}

getConfigWhenReady().then((config) => {
  setLanguage(config.ui_language);
});

button.addEventListener("click", () => invoke("cancel_current"));
