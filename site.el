;;; package -- Build script for site.
;;; Commentary:
;;;   Builds website by converting .org files into html.
;;; Code:
(require 'ox-publish)

(defun build-spore-site ()
  "Build spore website.

The static site is output into the site directory."
  (let ((norg-src-fontify-natively t)
        (org-publish-project-alist
         `(("spore" :components ("spore-main"))
           ("spore-main"
            :base-directory "./site"
            :publishing-function org-html-publish-to-html
            :publishing-directory "./target/site"
            :recursive t
            :auto-sitemap t
            :section-numbers t
            :sitemap-title "Spore"
            :sitemap-filename "index.org"
            :html-link-home "../"
            :html-link-up "../"
            )))
        (org-html-validation-link nil))
    (org-publish-project "spore")))

(defun build-spore-site-any-buffer ()
  "Build the spore site by switching to site.el."
  (with-current-buffer "site.el"
    (build-spore-site)))

(defun build-spore-after-save ()
  (interactive)
  (add-hook 'after-save-hook #'build-spore-site-any-buffer))

(build-spore-site)

(provide 'site)
;;; site.el ends here
