/**
 * 前端测试基座（PATCH002.65）：**只补 jsdom 缺项**，不 mock 业务逻辑。
 * 缺项来源 = 套件实际依赖（ResizeObserver / matchMedia / Web Animations）。
 */

// ResizeObserver：jsdom 无（滑杆 / 三角 / 阅读套件依赖）
class ResizeObserverStub {
  observe(): void {}
  unobserve(): void {}
  disconnect(): void {}
}
if (!("ResizeObserver" in globalThis)) {
  (globalThis as unknown as Record<string, unknown>).ResizeObserver = ResizeObserverStub;
}

// matchMedia：主题三态（跟随系统）查询依赖
if (!window.matchMedia) {
  const stub = (query: string): MediaQueryList =>
    ({
      matches: false,
      media: query,
      onchange: null,
      addListener: () => {},
      removeListener: () => {},
      addEventListener: () => {},
      removeEventListener: () => {},
      dispatchEvent: () => false,
    }) as unknown as MediaQueryList;
  window.matchMedia = stub as typeof window.matchMedia;
}

// document.timeline + Element.animate：Web Animations（删块动效驱动）依赖
if (!("timeline" in document)) {
  Object.defineProperty(document, "timeline", { value: { currentTime: 0 } });
}
if (!Element.prototype.animate) {
  Element.prototype.animate = function animate(): Animation {
    return {
      finished: Promise.resolve(),
      cancel: () => {},
      finish: () => {},
      addEventListener: () => {},
      removeEventListener: () => {},
    } as unknown as Animation;
  };
}
