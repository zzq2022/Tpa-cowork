# TPA CoWork Browser Control

## Short Description

A local bridge that lets the TPA CoWork desktop app control Chrome tabs you select or claim. Requires the TPA CoWork app.

## Detailed Description

TPA CoWork Browser Control connects Google Chrome to the TPA CoWork desktop app through Chrome Native Messaging. It lets TPA CoWork operate Chrome tabs that you create, select, or claim from the app, including page navigation, screenshots, form entry, frame-aware clicks, download observation, and emergency stop controls.

The extension does not work by itself. It requires the TPA CoWork native host installed on the same computer. When TPA CoWork is not running or the native host is not installed, the extension stays idle.

All-sites access is requested only so you can pick any page to control. TPA CoWork enforces per-tab ownership and its own approval checks before acting on a tab, so access alone never operates a page.

Visible controls:

- A page overlay appears while TPA CoWork controls a tab.
- The toolbar popup can stop control for the current tab or all controlled tabs.
- TPA CoWork Settings shows connection status, version diagnostics, and repair steps.

Data handling:

- Browser data is sent only to the local TPA CoWork app through Native Messaging.
- The extension does not send browsing data to a third-party server.
- TPA CoWork applies its own permission checks before real Chrome access and advanced actions such as observing or interrupting downloads and using raw Chrome DevTools Protocol commands.

## Category

Productivity

## Language

English
