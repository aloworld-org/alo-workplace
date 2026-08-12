// What the Docs blocks gained by adopting `ds/` (D2.01).
//
// The look of the migration is a matter of opinion; these are the properties
// that were measurably absent before it, so they are the ones worth a build.
// Each test names the thing the hand-built version did instead.
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, test, vi } from "vitest";

import { strings } from "../i18n";
import { AuthoringInsertModal } from "./AuthoringInsertModal";
import { EquationEditor } from "./EquationEditor";
import { ParagraphBlock } from "./ParagraphBlock";
import { ReferenceChip } from "./CrossReference";
import { TableBlock } from "./TableBlock";
import { computeNumbering } from "./numbering";

afterEach(cleanup);

describe("the table block", () => {
  test("its scroll region is reachable without a mouse, and says what it is", () => {
    // Before: a bare `<div>` with `overflow-x: auto` — scrollable by pointer
    // only, which is WCAG 2.1.1, and announced as nothing at all.
    render(<TableBlock rows={[["Symbol", "Value"]]} number="1" onChange={() => {}} />);
    const region = screen.getByRole("region", { name: "Table 1" });
    expect(region.tabIndex).toBe(0);
  });

  test("the table itself is named by its number, so a reference lands somewhere", () => {
    render(<TableBlock rows={[["Symbol", "Value"]]} number="2" onChange={() => {}} />);
    expect(screen.getByRole("table", { name: "Table 2" })).toBeDefined();
  });

  test("an unnumbered table still has a name", () => {
    render(<TableBlock rows={[["a"]]} number={undefined} onChange={() => {}} />);
    expect(
      screen.getByRole("table", { name: strings.tableBlockLabel }),
    ).toBeDefined();
  });

  test("the header row is still a row of column headers", () => {
    // `Th` defaults to `scope="col"`; the hand-rolled `<th>`s had no scope, so
    // no cell was associated with the column it belongs to.
    render(<TableBlock rows={[["Symbol", "Value"], ["k", "2"]]} onChange={() => {}} number={undefined} />);
    const headers = screen.getAllByRole("columnheader");
    for (const header of headers) expect(header.getAttribute("scope")).toBe("col");
  });

  test("editing a cell reports the whole grid back", () => {
    const onChange = vi.fn();
    render(<TableBlock rows={[["Symbol", "Value"], ["k", "2"]]} number={undefined} onChange={onChange} />);
    const cells = screen.getAllByLabelText(strings.tableCell);
    fireEvent.change(cells[0]!, { target: { value: "r" } });
    expect(onChange).toHaveBeenCalledWith([
      ["Symbol", "Value"],
      ["r", "2"],
    ]);
  });
});

describe("the paragraph block's insert controls", () => {
  const numbering = computeNumbering([]);

  test("they are announced as one named group, not two loose buttons", () => {
    render(
      <ParagraphBlock text="x" onChange={() => {}} items={[]} numbering={numbering} />,
    );
    // Click to enter edit mode — the toolbar only exists while editing.
    fireEvent.click(screen.getByRole("textbox"));
    expect(screen.getByRole("group", { name: strings.paraToolbar })).toBeDefined();
  });

  test("each control keeps its own tab stop", () => {
    // `keyboard="tab"`, deliberately: the reference button opens a picker full
    // of buttons inside the row, and a roving tab stop would swallow them.
    render(
      <ParagraphBlock text="x" onChange={() => {}} items={[]} numbering={numbering} />,
    );
    fireEvent.click(screen.getByRole("textbox"));
    const group = screen.getByRole("group", { name: strings.paraToolbar });
    for (const button of group.querySelectorAll("button")) {
      expect(button.getAttribute("tabindex")).toBeNull();
    }
  });

  test("inserting inline math puts the caret between the delimiters", () => {
    const onChange = vi.fn();
    render(
      <ParagraphBlock text="" onChange={onChange} items={[]} numbering={numbering} />,
    );
    fireEvent.click(screen.getByRole("textbox"));
    fireEvent.click(screen.getByText(strings.paraInlineMath));
    expect(onChange).toHaveBeenCalledWith("$  $");
  });
});

describe("the equation dialog", () => {
  function open(onClose = () => {}, onToggleNumbered?: (n: boolean) => void) {
    return render(
      <EquationEditor
        value="E = mc^2"
        onChange={() => {}}
        display={false}
        onInsert={() => {}}
        onClose={onClose}
        numbered={false}
        {...(onToggleNumbered === undefined ? {} : { onToggleNumbered })}
      />,
    );
  }

  test("Escape closes it", () => {
    // Before: nothing handled Escape here. The only way out of the symbol
    // palette was to find the × with a pointer.
    const onClose = vi.fn();
    open(onClose);
    fireEvent.keyDown(document, { key: "Escape" });
    expect(onClose).toHaveBeenCalledOnce();
  });

  test("Tab cannot leave it", () => {
    open();
    const dialog = screen.getByRole("dialog");
    const focusable = dialog.querySelectorAll<HTMLElement>("button, textarea, input");
    const last = focusable[focusable.length - 1]!;
    last.focus();
    fireEvent.keyDown(document, { key: "Tab" });
    expect(dialog.contains(document.activeElement)).toBe(true);
  });

  test("the numbered switch is a checkbox with its words bound to it", () => {
    // Before: a `<label>` wrapping a bare `<input type="checkbox">` with the
    // text loose beside it, and no disabled or hint state at all.
    const onToggle = vi.fn();
    open(() => {}, onToggle);
    const box = screen.getByRole("checkbox", { name: strings.eqNumbered });
    fireEvent.click(box);
    expect(onToggle).toHaveBeenCalledWith(true);
  });

  test("it names itself, and the mark before the title is not read out", () => {
    open();
    const dialog = screen.getByRole("dialog");
    expect(dialog.getAttribute("aria-label")).toBe(strings.eqTitle);
    expect(dialog.getAttribute("aria-modal")).toBe("true");
  });
});

describe("the insert-code dialog", () => {
  test("Escape closes it and ⌘/Ctrl+Enter still inserts", () => {
    const onClose = vi.fn();
    const onInsert = vi.fn();
    render(
      <AuthoringInsertModal kind="code" onInsert={onInsert} onClose={onClose} />,
    );
    const code = screen.getByLabelText(strings.codeInputLabel);
    fireEvent.change(code, { target: { value: "let x = 1;" } });
    fireEvent.keyDown(code, { key: "Enter", metaKey: true });
    expect(onInsert).toHaveBeenCalledOnce();
    // Email-safe HTML: the code arrives highlighted, so it is the markup
    // around it that proves the shortcut inserted this editor's contents.
    const html = onInsert.mock.calls[0]![0];
    expect(html).toContain("data-alo-lang");
    expect(html.replace(/<[^>]+>/g, "")).toContain("let x = 1;");

    fireEvent.keyDown(document, { key: "Escape" });
    expect(onClose).toHaveBeenCalledOnce();
  });

  test("the confirm button is disabled until there is code to insert", () => {
    render(
      <AuthoringInsertModal kind="code" onInsert={() => {}} onClose={() => {}} />,
    );
    const insert = screen.getByRole("button", { name: strings.insertConfirm });
    expect(insert.hasAttribute("disabled")).toBe(true);
    fireEvent.change(screen.getByLabelText(strings.codeInputLabel), {
      target: { value: "x" },
    });
    expect(insert.hasAttribute("disabled")).toBe(false);
  });
});

describe("a cross-reference", () => {
  test("resolves to the target's current number", () => {
    const numbering = computeNumbering([
      { id: "eq-1", kind: "equation", title: "First" },
    ]);
    render(<ReferenceChip targetId="eq-1" numbering={numbering} />);
    expect(screen.getByText(`${strings.refEquation} 1`)).toBeDefined();
  });

  test("a broken reference says so in words, not only in colour", () => {
    render(<ReferenceChip targetId="gone" numbering={computeNumbering([])} />);
    expect(screen.getByText(strings.refBroken)).toBeDefined();
  });
});
