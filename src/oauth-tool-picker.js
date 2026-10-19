const tool = document.getElementById("tool");
const custom = document.getElementById("custom_tool");
if (tool && custom) {
  tool.addEventListener("change", () => {
    custom.style.display = tool.value === "__custom__" ? "block" : "none";
  });
}
