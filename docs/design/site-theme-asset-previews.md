# Site theme asset previews

## Surface

Logo and favicon cards render the selected image beside their description and
replace/remove actions. New uploads preview immediately from the local file;
saved assets use an authenticated download. Logos use a navigation-shaped
frame; favicons use a compact square frame.

Descriptions live in the standard accessible information popover so each card
stays focused on its preview, state, and available actions.

Both cards accept files from the picker or drag-and-drop. After the blob upload,
Sites registers the image in the source-linked `Website / Identity` Drive
folder before updating the card.

An empty card contains a full-width dashed drop zone with explicit drag and
drop and device-picker instructions. Once populated, it contracts to the image
preview and replace/remove actions so persistent guidance does not become
visual noise.

## Errors

A preview download failure leaves the asset marked as uploaded and keeps its
replace and remove actions available. Upload failures continue through the
existing Sites error banner.

## Tenancy

Previews use the existing authenticated JMAP download template and account ID.
The Identity attachment endpoint requires ownership of both the website and
blob; another tenant receives the same not-found response as a missing record.
No public asset URL is constructed.

## Out of scope

Cropping, image editing, and changing the stored asset format are unchanged.

The rejected alternative was displaying the blob identifier or constructing a
public URL; neither proves what image the user selected, and the latter would
bypass the authenticated Drive boundary.
