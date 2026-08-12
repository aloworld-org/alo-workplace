// An editable table block (ADR 0015). Row 0 is the header. Tables are numbered
// items, so they can be cross-referenced ("Table 1"). Add/remove rows and
// columns; every cell is editable.
import { Plus, Trash2 } from "lucide-react";

import { strings } from "../i18n";
import { Table, Td, Th } from "../ds";
import styles from "./TableBlock.module.css";

interface TableBlockProps {
  rows: string[][];
  /** The table's number from the engine, shown as a caption. */
  number: string | undefined;
  onChange: (rows: string[][]) => void;
}

export function TableBlock({ rows, number, onChange }: TableBlockProps) {
  const cols = rows[0]?.length ?? 0;

  function setCell(r: number, c: number, value: string) {
    const next = rows.map((row) => [...row]);
    const target = next[r];
    if (target !== undefined) target[c] = value;
    onChange(next);
  }

  function addRow() {
    onChange([...rows, Array.from({ length: cols }, () => "")]);
  }

  function removeRow(r: number) {
    if (rows.length <= 1) return;
    onChange(rows.filter((_, i) => i !== r));
  }

  function addColumn() {
    onChange(rows.map((row) => [...row, ""]));
  }

  function removeColumn(c: number) {
    if (cols <= 1) return;
    onChange(rows.map((row) => row.filter((_, i) => i !== c)));
  }

  // The table's name, which the hand-rolled version never had: a numbered table
  // is known by its number, because that is what a cross-reference points at.
  const label =
    number === undefined
      ? strings.tableBlockLabel
      : `${strings.refTable} ${number}`;

  return (
    <figure className={styles.figure}>
      <Table label={label} grid flat>
        <thead>
          <tr>
            {(rows[0] ?? []).map((cell, c) => (
              <Th key={c} className={styles.head}>
                <input
                  className={styles.cellInput}
                  value={cell}
                  onChange={(e) => setCell(0, c, e.target.value)}
                  aria-label={strings.tableHeaderCell}
                />
                <button
                  type="button"
                  className={styles.colRemove}
                  onClick={() => removeColumn(c)}
                  title={strings.tableRemoveColumn}
                  aria-label={strings.tableRemoveColumn}
                >
                  <Trash2 size={12} />
                </button>
              </Th>
            ))}
            {/* A control column, not a heading — `hideLabel` would hide the
                button itself, so the name it carries is the button's own. */}
            <Th className={styles.addColCell}>
              <button
                type="button"
                className={styles.addBtn}
                onClick={addColumn}
                title={strings.tableAddColumn}
                aria-label={strings.tableAddColumn}
              >
                <Plus size={14} />
              </button>
            </Th>
          </tr>
        </thead>
        <tbody>
          {rows.slice(1).map((row, i) => {
            const r = i + 1;
            return (
              <tr key={r}>
                {row.map((cell, c) => (
                  <Td key={c}>
                    <input
                      className={styles.cellInput}
                      value={cell}
                      onChange={(e) => setCell(r, c, e.target.value)}
                      aria-label={strings.tableCell}
                    />
                  </Td>
                ))}
                <Td className={styles.rowRemoveCell}>
                  <button
                    type="button"
                    className={styles.rowRemove}
                    onClick={() => removeRow(r)}
                    title={strings.tableRemoveRow}
                    aria-label={strings.tableRemoveRow}
                  >
                    <Trash2 size={12} />
                  </button>
                </Td>
              </tr>
            );
          })}
        </tbody>
      </Table>
      <div className={styles.footerRow}>
        <button type="button" className={styles.addRowBtn} onClick={addRow}>
          <Plus size={14} />
          {strings.tableAddRow}
        </button>
        {number !== undefined && (
          <figcaption className={styles.caption}>
            {strings.refTable} {number}
          </figcaption>
        )}
      </div>
    </figure>
  );
}
