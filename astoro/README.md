# Astro Starter Kit: Minimal

```sh
npm create astro@latest -- --template minimal
```

> 🧑‍🚀 **Seasoned astronaut?** Delete this file. Have fun!

## 🚀 Project Structure

Inside of your Astro project, you'll see the following folders and files:

```text
/
├── public/
├── src/
│   └── pages/
│       └── index.astro  # WebAssembly test page
└── package.json
```

Astro looks for `.astro` or `.md` files in the `src/pages/` directory. Each page is exposed as a route based on its file name.

## 🧪 WebAssembly Testing

This project includes a test page for WebAssembly functionality:

- **URL**: `http://localhost:4321` (when running `npm run dev`)
- **Features**:
  - Initialize WebAssembly engine
  - Add RDF triples to knowledge base
  - Clear knowledge base
  - Render knowledge graph visualization
  - Real-time logging output

### Testing Steps:

1. Start the development server: `npm run dev`
2. Open `http://localhost:4321` in your browser
3. The page will automatically load the WebAssembly module and display the Fukurow OWL reasoning engine interface
4. Test various functionalities:
   - **Engine Selection**: Choose between OWL Lite, OWL DL, or RDFS reasoning
   - **Execution Mode**: Select consistency checking, classification, or SPARQL queries
   - **RDF Input**: Enter Turtle-formatted RDF/OWL data
   - **Run**: Execute the selected reasoning operation
   - **Results**: View the reasoning results and knowledge graph visualization

### ✅ Verification Complete

The WebAssembly functionality has been successfully verified. The fukurow-wasm crate is published on crates.io and can be used in web applications for RDF/OWL reasoning operations.

Any static assets, like images, can be placed in the `public/` directory.

## 🧞 Commands

All commands are run from the root of the project, from a terminal:

| Command                   | Action                                           |
| :------------------------ | :----------------------------------------------- |
| `npm install`             | Installs dependencies                            |
| `npm run dev`             | Starts local dev server at `localhost:4321`      |
| `npm run build`           | Build your production site to `./dist/`          |
| `npm run preview`         | Preview your build locally, before deploying     |
| `npm run astro ...`       | Run CLI commands like `astro add`, `astro check` |
| `npm run astro -- --help` | Get help using the Astro CLI                     |

## 👀 Want to learn more?

Feel free to check [our documentation](https://docs.astro.build) or jump into our [Discord server](https://astro.build/chat).
