import type { Config } from "@docusaurus/types"
import type * as Preset from "@docusaurus/preset-classic"

const config: Config = {
  title: "Rugpi",
  tagline:
    "An open-source platform empowering you to build innovative products based on Raspberry Pi.",
  url: "https://oss.silitics.com/",
  baseUrl: "/rugpi/",

  onBrokenLinks: "warn",
  onBrokenMarkdownLinks: "warn",

  // We do not care about old browsers not supporting SVG.
  favicon: "/img/logo.svg",

  organizationName: "silitics",
  projectName: "rugpi",

  i18n: {
    defaultLocale: "en",
    locales: ["en"],
  },

  presets: [
    [
      "classic",
      {
        docs: {
          sidebarPath: require.resolve("./sidebars.js"),
          lastVersion: "0.6",
          editUrl: "https://github.com/silitics/rugpi/tree/main/www/",
        },
        blog: {
          showReadingTime: true,
          editUrl: "https://github.com/silitics/rugpi/tree/main/www/",
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

    announcementBar: {
      // id: "release",
      content: `
          We are excited to announce the release of Rugpi version 0.6! 🎉
          Read the <a href="/rugpi/blog/releases/0.6">release blog post</a>.
        `,
      backgroundColor: "#bdddfb",
      // backgroundColor: "#bdddfb",
      // textColor: "#000000",
      isCloseable: false,
    },
    navbar: {
      title: "Rugpi",
      logo: {
        alt: "Rugpi Logo",
        src: "img/logo.svg",
      },
      items: [
        {
          type: "doc",
          docId: "getting-started",
          position: "left",
          label: "Docs",
        },
        {
          to: "/devices",
          label: "Supported Devices",
          position: "left",
        },
        { to: "/blog", label: "Blog", position: "left" },
        {
          type: "docsVersionDropdown",
          position: "right",
          // dropdownItemsAfter: [{to: '/versions', label: 'All versions'}],
          dropdownActiveClassDisabled: true,
        },
        {
          href: "https://github.com/silitics/rugpi",
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
              href: "https://github.com/silitics/rugpi",
            },
            {
              label: "Discussions",
              href: "https://github.com/silitics/rugpi/discussions",
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
      copyright: `<div>Made with ❤️ for OSS</div><div>Copyright © ${new Date().getFullYear()} <a href="https://silitics.com">Silitics GmbH</a></div><div>Built with Docusaurus</div><div style="margin-top: 0.5em"><small>Raspberry Pi is a trademark of Raspberry Pi Ltd</small></div>`,
    },
    prism: {
      theme: require("prism-react-renderer").themes.vsDark,
      additionalLanguages: ["rust", "toml"],
    },
  } satisfies Preset.ThemeConfig,

  plugins: [
    async function tailwind(context, options) {
      return {
        name: "docusaurus-tailwindcss",
        configurePostCss(postcssOptions) {
          postcssOptions.plugins.push(require("tailwindcss"));
          postcssOptions.plugins.push(require("autoprefixer"));
          return postcssOptions;
        },
      };
    },
  ]
}

export default config
