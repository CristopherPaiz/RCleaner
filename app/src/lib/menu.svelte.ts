// Menú contextual propio. El de WebView2 (Atrás, Recargar, Inspeccionar…) está
// desactivado en toda la app; cada zona ofrece aquí sus propias acciones.

export interface MenuItem {
  label: string;
  icon?: string;
  action?: () => void | Promise<void>;
  danger?: boolean;
  disabled?: boolean;
  /** Texto secundario a la derecha (atajo, aviso…). */
  hint?: string;
}

/** `"-"` = separador. */
export type MenuEntry = MenuItem | "-";

class MenuStore {
  open = $state<{ x: number; y: number; items: MenuEntry[] } | null>(null);

  /** Abre el menú en la posición del clic derecho. */
  show(e: MouseEvent, items: MenuEntry[]) {
    e.preventDefault();
    e.stopPropagation();
    const clean = items.filter((it, i, all) => it !== "-" || (i > 0 && i < all.length - 1 && all[i - 1] !== "-"));
    if (clean.length === 0) return;
    this.open = { x: e.clientX, y: e.clientY, items: clean };
  }

  close() {
    this.open = null;
  }
}

export const menu = new MenuStore();

/** Menú de edición para campos de texto (cortar, copiar, pegar…). */
export function textMenu(el: HTMLInputElement | HTMLTextAreaElement, t: (k: string) => string): MenuEntry[] {
  const sel = (el.selectionEnd ?? 0) > (el.selectionStart ?? 0);
  const editable = !el.readOnly && !el.disabled;
  const run = (cmd: string) => () => {
    el.focus();
    document.execCommand(cmd);
  };
  return [
    { label: t("ui.menu.cut"), icon: "x", disabled: !sel || !editable, action: run("cut"), hint: "Ctrl+X" },
    { label: t("ui.menu.copy"), icon: "copy", disabled: !sel, action: run("copy"), hint: "Ctrl+C" },
    {
      label: t("ui.menu.paste"),
      icon: "file",
      disabled: !editable,
      hint: "Ctrl+V",
      action: async () => {
        el.focus();
        try {
          const text = await navigator.clipboard.readText();
          document.execCommand("insertText", false, text);
        } catch {
          document.execCommand("paste");
        }
      },
    },
    "-",
    { label: t("ui.menu.select_all"), icon: "check", action: () => el.select(), hint: "Ctrl+A" },
  ];
}
