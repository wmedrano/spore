;;; spore-mode.el --- Major mode for Spore.
;;; Commentary:
;;; Provides a major mode for the Spore programming language.
;;; Code:

(defvar spore-mode-syntax-table
  (let ((table (make-syntax-table)))
    (modify-syntax-entry ?\( "()" table)
    (modify-syntax-entry ?\) ")(" table)
    (modify-syntax-entry ?\; "<" table)
    (modify-syntax-entry ?\n ">" table)
    (modify-syntax-entry ?\" "\"" table)
    (modify-syntax-entry ?' "\'" table)
    table)
  "Syntax table for `spore-mode`.")

(defconst spore-font-lock-keywords
  (let ((keywords '("if" "define" "defun" "let*" "return" "for" "and" "or" "quote" "function"))
        (constants '("true" "false" "nil")))
    `((,(regexp-opt keywords 'symbols) . font-lock-keyword-face)
      (,(regexp-opt constants 'symbols) . font-lock-constant-face)
      ;; highlight function names
      ("(\\(defun\\|define\\)[ \t]+\\([_a-zA-Z0-9-]+\\)" 2 font-lock-function-name-face)))
  "Font lock keywords for `spore-mode`.")

;;;###autoload
(define-derived-mode spore-mode lisp-mode "Spore"
  "Major mode for editing Spore files."
  :syntax-table spore-mode-syntax-table
  (setq-local font-lock-defaults '((spore-font-lock-keywords)))
  (setq-local comment-start ";"))

;;;###autoload
(add-to-list 'auto-mode-alist '("\\.sp\\'" . spore-mode))

(defvar spore-bin "spore")

(defun org-babel-execute:spore (body params)
  "Execute block BODY as a Spore program in Org Babel.

PARAMS: Alist of parameters, supporting:
- :cmd The Spore executable command (defaults to \"spore\").
- :cmdline Additional command-line arguments to pass to the Spore executable (defaults to \"\")."
  (let* ((cmd (or (cdr (assoc :cmd params)) spore-bin))
         (cmdline (or (cdr (assoc :cmdline params)) "")))
    (org-babel-eval (format "%s %s" cmd cmdline) body)))

(with-eval-after-load 'ob
  (add-to-list 'org-babel-load-languages '(spore . t)))

(provide 'spore-mode)
;;; spore-mode.el ends here.
