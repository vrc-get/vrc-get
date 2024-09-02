import ResizeObserver from "resize-observer-polyfill";

if (typeof window !== "undefined") {
	if (typeof window.ResizeObserver === "undefined") {
		//window.ResizeObserver = (await import("resize-observer-polyfill")).default;
		window.ResizeObserver = ResizeObserver;
	}
}

export default {};
