// Insert-into-email modal (ADR 0015): lets compose insert an equation or a code
// block into the message body. Reuses the equation modal and the dark code
// editor, and emits email-safe HTML (MathML for math, inline-styled <pre> for
// code) that the compose editor drops in at the caret. Lazy-loaded, so KaTeX/
// Prism stay off the mail path until a user actually inserts one.
import { useState } from "react";
import { Code2 } from "lucide-react";

import { strings } from "../i18n";
import { Button, Modal } from "../ds";
import { EquationEditor } from "./EquationEditor";
import { CodeBlock } from "./CodeBlock";
import { DEFAULT_LANGUAGE } from "./prism";
import { codeEmailHtml, equationEmailHtml } from "./emailBlocks";
import styles from "./AuthoringInsertModal.module.css";

interface InsertProps {
  kind: "equation" | "code";
  /** Called with the email-safe HTML to insert at the caret. */
  onInsert: (html: string) => void;
  onClose: () => void;
}

function EquationInsert({ onInsert, onClose }: Omit<InsertProps, "kind">) {
  const [latex, setLatex] = useState("");
  return (
    <EquationEditor
      value={latex}
      onChange={setLatex}
      display={false}
      onInsert={() => onInsert(equationEmailHtml(latex, false))}
      onClose={onClose}
    />
  );
}

function CodeInsert({ onInsert, onClose }: Omit<InsertProps, "kind">) {
  const [code, setCode] = useState("");
  const [language, setLanguage] = useState(DEFAULT_LANGUAGE);
  const canInsert = code.trim().length > 0;
  return (
    <Modal
      title={strings.codeInsertTitle}
      icon={<Code2 size={18} />}
      onClose={onClose}
      wide
      actions={<span className={styles.hint}>{strings.codeInsertHint}</span>}
      footer={
        <div className={styles.actions}>
          <Button variant="ghost" onClick={onClose}>
            {strings.insertCancel}
          </Button>
          <Button
            disabled={!canInsert}
            onClick={() => onInsert(codeEmailHtml(code, language))}
          >
            {strings.insertConfirm}
          </Button>
        </div>
      }
    >
      {/* Escape and the backdrop belong to `Modal` now; this is only the
          shortcut that confirms, which bubbles up from the code editor. */}
      <div
        onKeyDown={(e) => {
          if ((e.metaKey || e.ctrlKey) && e.key === "Enter" && canInsert) {
            onInsert(codeEmailHtml(code, language));
          }
        }}
      >
        <CodeBlock
          code={code}
          onChange={setCode}
          language={language}
          onLanguageChange={setLanguage}
          tall
        />
        {canInsert && (
          <div className={styles.previewWrap}>
            <span className={styles.previewLabel}>{strings.codePreviewLabel}</span>
            {/* Our own generated, code-escaped email HTML — safe to render. */}
            <div
              className={styles.preview}
              dangerouslySetInnerHTML={{ __html: codeEmailHtml(code, language) }}
            />
          </div>
        )}
      </div>
    </Modal>
  );
}

export function AuthoringInsertModal({ kind, onInsert, onClose }: InsertProps) {
  return kind === "equation" ? (
    <EquationInsert onInsert={onInsert} onClose={onClose} />
  ) : (
    <CodeInsert onInsert={onInsert} onClose={onClose} />
  );
}
