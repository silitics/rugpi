import type { Config } from "@docusaurus/types"
import type * as Preset from "@docusaurus/preset-classic"

const config: Config = {
  title: "Rugix",
  tagline:
    "A suite of open-source tools to build reliable embedded Linux devices with efficient and secure over-the-air update capabilities.",
  url: "https://oss.silitics.com/",
  baseUrl: "/rugix/",

  onBrokenLinks: "warn",
  onBrokenMarkdownLinks: "warn",

  // We do not care about old browsers not supporting SVG.
  favicon: "/img/logo.svg",

  organizationName: "silitics",
  projectName: "rugix",

  i18n: {
    defaultLocale: "en",
    locales: ["en"],
  },

  markdown: {
    mermaid: true,
  },

  presets: [
    [
      "classic",
      {
        docs: {
          sidebarPath: require.resolve("./sidebars.js"),
          lastVersion: "current",
          // lastVersion: "0.7.5",
          editUrl: "https://github.com/silitics/rugix/tree/main/www/",
        },
        blog: {
          showReadingTime: true,
          editUrl: "https://github.com/silitics/rugix/tree/main/www/",
        },
        theme: {
          customCss: require.resolve("./src/css/custom.css"),
        },
      } satisfies Preset.Options,
    ],
  ],

  themeConfig: {
    colorMode: {
      defaultMode: "dark",
      disableSwitch: true,
      respectPrefersColorScheme: false,
    },

    // announcementBar: {
    //   // id: "release",
    //   content: `
    //       We are excited to announce the release of Rugix version 0.7! 🎉
    //       Read the <a href="/rugix/blog/releases/0.7">release blog post</a>.
    //     `,
    //   backgroundColor: "#bdddfb",
    //   // backgroundColor: "#bdddfb",
    //   // textColor: "#000000",
    //   isCloseable: false,
    // },
    navbar: {
      title: "Rugix",
      logo: {
        alt: "Rugix Logo",
        src: "img/logo.svg",
      },
      items: [
        {
          to: "/devices",
          label: "Supported Devices",
          position: "left",
        },
        {
          type: "doc",
          docId: "the-rugix-project",
          label: "Why Rugix?",
          position: "left",
        },
        {
          type: "doc",
          docId: "getting-started",
          position: "left",
          label: "Docs",
        },
        { to: "/blog", label: "Blog", position: "left" },
        {
          to: "/commercial-support",
          label: "Commercial Support",
          position: "right",
        },
        {
          type: "docsVersionDropdown",
          position: "right",
          // dropdownItemsAfter: [{to: '/versions', label: 'All versions'}],
          dropdownActiveClassDisabled: true,
        },
        {
          href: "https://github.com/silitics/rugix",
          position: "right",
          className: "header-github-link",
          "aria-label": "GitHub",
        },
      ],
    },
    footer: {
      style: "dark",
      links: [
        {
          title: "Docs",
          items: [
            {
              label: "Getting Started",
              to: "/docs/getting-started",
            },
            {
              label: "User Guide",
              to: "/docs/guide",
            },
          ],
        },
        {
          title: "Community",
          items: [
            {
              label: "GitHub",
              href: "https://github.com/silitics/rugix",
            },
            {
              label: "Discord",
              href: "https://discord.gg/cZ8wP9jNsn",
            },
          ],
        },
        {
          title: "More",
          items: [
            {
              label: "Blog",
              to: "/blog",
            },
          ],
        },
        {
          title: "Legal",
          items: [
            {
              // German and EU law require us to have a privacy policy.
              label: "Privacy Policy",
              href: "https://silitics.com/privacy-policy",
            },
            {
              // German law requires us to have an Impressum.
              label: "Impressum",
              href: "https://silitics.com/impressum",
            },
          ],
        },
      ],
      copyright: `<div>Made with ❤️ for OSS</div><div>Copyright © ${new Date().getFullYear()} <a href="https://silitics.com">Silitics GmbH</a></div><div><small>Built with Docusaurus</small></div>`,
    },
    prism: {
      theme: require("prism-react-renderer").themes.vsDark,
      additionalLanguages: ["rust", "toml", "yaml", "bash", "docker"],
    },
  } satisfies Preset.ThemeConfig,

  themes: ["@docusaurus/theme-mermaid", "docusaurus-json-schema-plugin"],

  plugins: [
    async function tailwind(context, options) {
      return {
        name: "docusaurus-tailwindcss",
        configurePostCss(postcssOptions) {
          postcssOptions.plugins.push(require("tailwindcss"))
          postcssOptions.plugins.push(require("autoprefixer"))
          return postcssOptions
        },
      }
    },
  ],
}

export default config
