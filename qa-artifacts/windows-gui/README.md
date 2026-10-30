# Jusage V0.3 Windows GUI evidence

The PNGs in this folder are safe-to-share Windows GUI captures. They were copied from interactive session 1 with `CopyFromScreen`; they are not `--preview` BMP renders.

Capture environment: one 1920×1080 display, 1920×1032 work area, 96 DPI (100%). The settings screenshot is from the live `--isolated` app and contains no usage values. The other screenshots are from a separate temporary Windows Release fixture: actual Windows UI pixels, synthetic usage/error/stale data, separate QA profile, no network worker, and no Windows or WSL authentication reads.

Raw live usage captures showed account usage, plan, or reset time. They remain in the ignored local-only `qa-artifacts/private-local/` folder and are not part of this commit or PR. Offscreen `--preview` BMP renders are also excluded.

| File | Evidence | Data |
| --- | --- | --- |
| `02-two-windows-synthetic.png` | Both usage windows in the compact popup; 392×626 | Synthetic, 58% and 82% |
| `03-details-expanded-top-synthetic.png` | Expanded details at Home; 392×1016 | Synthetic |
| `04-details-page-down-synthetic.png` | Details at the lower scroll limit; fixed controls remain visible; 392×1016 | Synthetic |
| `05-details-mouse-wheel-synthetic.png` | Details after mouse-wheel scroll; 392×1016 | Synthetic |
| `06-settings-dark-live.png` | Settings screen in Midnight Slate; 392×562 | Live app settings; no usage data |
| `08-error-state-synthetic.png` | Fetch error state; 392×262 | Synthetic network error |
| `09-stale-state-synthetic.png` | Both windows with stale-data notice; 392×682 | Synthetic, 11 minutes old |
| `10-details-folded-synthetic.png` | Details folded back to the compact popup; 392×626 | Synthetic |

The included captures were reviewed for clipping, overlap, contrast, text legibility, and visible credentials/account identifiers. No credentials or account identifiers appear. Actual 125%/150% DPI and dual-monitor placement were not captured or tested.
