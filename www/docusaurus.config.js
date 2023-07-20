/** @type {import('@docusaurus/types').Config} */
const config = {
  title: "Rugpi",
  tagline: "An open-source platform empowering you to build innovative products based on Raspberry Pi.",
  url: "https://oss.silitics.com/",
  baseUrl: "/rugpi/",

  onBrokenLinks: "warn",
  onBrokenMarkdownLinks: "warn",

  // We do not care about old browsers not supporting SVG.
  favicon: "img/logo.svg",

  organizationName: "silitics",
  projectName: "rugpi",

  i18n: {
    defaultLocale: "en",
    locales: ["en"],
  },

  presets: [
    [
      "classic",
      /** @type {import('@docusaurus/preset-classic').Options} */
      ({
        docs: {
          sidebarPath: require.resolve("./sidebars.js"),
          editUrl: "https://github.com/silitics/rugpi/tree/main/www/",
        },
        blog: {
          showReadingTime: true,
          editUrl: "https://github.com/silitics/rugpi/tree/main/www/",
        },
        theme: {
          customCss: require.resolve("./src/css/custom.css"),
        },
      }),
    ],
  ],

  themeConfig:
    /** @type {import('@docusaurus/preset-classic').ThemeConfig} */
    ({
      colorMode: {
        defaultMode: "dark",
        disableSwitch: true,
        respectPrefersColorScheme: false,
      },
      
      announcementBar: {
        id: "under_construction",
        content:
          "🚨 <strong>EXPERIMENTAL</strong>: Rugpi <strong>is still experimental</strong>. Expect things to change and break. Do not use in production just yet! 🚨",
        backgroundColor: "#FFFF00",
        textColor: "#000000",
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
          { to: "/blog", label: "Blog", position: "left" },
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
                href: "https://silitics.com/privacy-policy"
              },
              {
                // German law requires us to have an Impressum.
                label: "Impressum",
                href: "https://silitics.com/impressum",
              },
            ],
          },
        ],
        copyright: `<div>Made with ❤️ for OSS</div><div>Copyright © ${new Date().getFullYear()} <a href="https://silitics.com">Silitics GmbH</a></div><div>Built with Docusaurus</div>`,
      },
      prism: {
        theme: require("prism-react-renderer/themes/oceanicNext"),
        additionalLanguages: ["rust", "toml"],
      },
    }),
}

module.exports = config
