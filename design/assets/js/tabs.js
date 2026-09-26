// ===== 页签（uiverse glider 组件：radio 驱动滑块，change 同步页面显隐） =====
document.querySelectorAll('.tabs input[type="radio"]').forEach((radio) => {
  radio.addEventListener("change", () => {
    rollbackDelConfirms(); // 换页收口未决确认：切走的行开盖态不可见且 mouseout 不再来
    for (const page of ["todos", "bubbles", "whiteboard"]) {
      $(`page-${page}`).hidden = page !== radio.dataset.page;
    }
    setArchiveVisible(radio.dataset.page === "todos"); // 归档仅清单页生效，出入场动画编排
    for (const g of glassBars) g.sync(); // 换页后重算各浮钮（白板页显示时才有几何）
    syncHints(); // 换页后重算边缘三角（显隐随清单可滚动性与几何）
  });
});
