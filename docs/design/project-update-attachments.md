# Project update attachments

**Surface.** `POST /projects/updates` accepts up to eight uploaded blob references
(`blobId`, `filename`, `size`); list responses return the same attachment records.
The project overview uploads bytes through the existing authenticated JMAP blob
endpoint before publishing and downloads them through the tenant-scoped blob URL.

**Errors.** Empty identifiers or names, negative sizes, and more than eight files
return HTTP 422. Upload or publish failures remain visible in the composer and do
not clear its text or selected files.

**Tenancy.** Update reads and writes retain the project visibility predicate. Blob
downloads are independently authenticated and account-scoped, so a reference
cannot grant access to another tenant's bytes.

**Out of scope.** Inline image editing, captions, and Drive selection are not part
of this increment. We rejected embedding data URLs in update text because that
would bypass blob lifecycle, size controls, and authenticated downloads.
