const { invoke } = window.__TAURI__.core;
const button = document.getElementById("cancel");

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
  const english = config.ui_language === "en" || (config.ui_language === "system" && !navigator.language.toLowerCase().startsWith("zh"));
  button.innerHTML = english ? "Esc&nbsp;&nbsp;Cancel" : "Esc&nbsp;&nbsp;取消";
  button.setAttribute("aria-label", english ? "Cancel recognition" : "取消识别");
});

button.addEventListener("click", () => invoke("cancel_current"));
