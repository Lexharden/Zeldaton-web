# Artwork slots

Drop image files here using these names. Anything missing is skipped silently (the site
falls back to its CSS-only decoration), so you can add them one by one.

| File                       | Where it appears                          | Suggested size      |
| -------------------------- | ----------------------------------------- | ------------------- |
| `hero-bg.jpg`              | Hero background (dark, wide landscape)    | 1920x1080+          |
| `hero-link.png`            | Hero character cutout (transparent PNG)   | ~900px tall         |
| `hyrule-shield.png`        | Footer / HiveShock accents (transparent)  | ~600px              |
| `master-sword.png`         | Section accents (transparent, vertical)   | ~800px tall         |
| `bg-forest.jpg`            | Racers section background                 | 1920x1080+          |
| `bg-field.jpg`             | Streams section background                | 1920x1080+          |
| `bg-temple.jpg`            | HiveShock section background              | 1920x1080+          |
| `bg-castle.jpg`            | Footer background                         | 1920x1080+          |
| `items/<item-id>.png`      | Item icons, ids in `src/config/event.ts`  | ~128px, transparent |

Item ids: master-sword, hookshot, longshot, bow, bombs, boomerang, megaton-hammer,
iron-boots, mirror-shield.

Item pictures (`items/`) are referenced from the catalog by **file name only**
(admin panel → Catalog → Icon). The backend serves them from `/api/media/items/<name>`; pictures
uploaded in the panel are stored on the server and need no commit. Keep new files here at most
256 px on the longest side, with a transparent background.

Names can be changed in `src/config/artwork.ts`.
