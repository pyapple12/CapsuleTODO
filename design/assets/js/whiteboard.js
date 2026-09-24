// ===== 白板（防抖自动保存状态行——实验场仅模拟状态，不落盘） =====
$("wb-board").addEventListener("input", () => {
  if ($("wb-board").value === wbSaved) {
    $("wb-status").hidden = true;
    return;
  }
  $("wb-status").hidden = false;
  $("wb-status").textContent = "编辑中…";
  window.clearTimeout(wbTimer);
  wbTimer = window.setTimeout(() => {
    wbSaved = $("wb-board").value;
    $("wb-status").textContent = "✓ 已自动保存";
  }, 800);
});
