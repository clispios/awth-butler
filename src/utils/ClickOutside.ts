import { Accessor, onCleanup } from "solid-js";

// @ts-ignore TS6133
function clickOutside(el: HTMLElement, accessor: Accessor<() => void>) {
  const onClick = (e: MouseEvent) => {
    if (!el.contains(e.target as Node)) {
      accessor()?.();
    }
  };
  document.body.addEventListener("click", onClick);

  onCleanup(() => {
    document.body.removeEventListener("click", onClick);
  });
}

declare module "solid-js" {
  namespace JSX {
    interface Directives {
      clickOutside: () => void;
    }
  }
}

export default clickOutside;
