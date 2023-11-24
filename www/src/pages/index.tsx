import React from "react"
import clsx from "clsx"
import Link from "@docusaurus/Link"
import useDocusaurusContext from "@docusaurus/useDocusaurusContext"
import Layout from "@theme/Layout"
import HomepageFeatures from "@site/src/components/HomepageFeatures"
import Admonition from "@theme/Admonition"

import styles from "./index.module.css"

function HomepageHeader() {
  return (
    <header
      className={clsx("hero hero--primary", styles.heroBanner, styles.hero)}
    >
      <div className="container">
        <h1 className="hero__title">
          The Commercial-Grade Platform for Raspberry Pi
        </h1>
        <p className="hero__subtitle">
          Rugpi is an open-source platform empowering you to create innovative
          products based on Raspberry Pi.
        </p>
        <p style={{ maxWidth: "80ch", margin: "1.5em auto" }}>
          Rugpi enables you to{" "}
          <strong>
            build commercial-grade, customized variants of{" "}
            <a href="https://www.raspberrypi.com/software/">Raspberry Pi OS</a>{" "}
          </strong>
          for your project. It boasts three core features: (1) A modern workflow
          to build customized system images, (2) robust{" "}
          <strong>over-the-air updates with rollback support</strong> of the
          entire system, including firmware files, and (3){" "}
          <strong>managed state</strong> which is preserved across reboots and
          updates.
        </p>
        <div className={styles.buttons}>
          <Link
            className="button button--secondary button--lg"
            to="/docs/getting-started"
          >
            Get Started 🚀
          </Link>
        </div>
      </div>
    </header>
  )
}

export default function Home(): JSX.Element {
  const { siteConfig } = useDocusaurusContext()
  return (
    <Layout title="Home" description={siteConfig.tagline}>
      <HomepageHeader />
      <main>
        <div style={{ maxWidth: "80ch", padding: "2rem 0", margin: "0 auto" }}>
          <Admonition type="info" title="Stability Guarantees">
            <p>
              While Rugpi is a young and evolving project, we understand that the lifetime of embedded devices spans multiple years, if not decades. Backwards incompatible changes to the update mechanism will be made only after careful consideration and consultation with our users. This ensures that devices using Rugpi can be updated in the future. If you're developing integrations with Rugpi, please be aware that the CLI and APIs are still expected to change.
            </p>
          </Admonition>
        </div>
        <HomepageFeatures />
      </main>
    </Layout>
  )
}
