import React from "react"
import clsx from "clsx"
import Link from "@docusaurus/Link"
import useDocusaurusContext from "@docusaurus/useDocusaurusContext"
import Layout from "@theme/Layout"
import HomepageFeatures from "@site/src/components/HomepageFeatures"

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
            <a href="https://en.wikipedia.org/wiki/Raspberry_Pi_OS">
              Raspberry Pi OS
            </a>{" "}
          </strong>
          for your product. It boasts two core features: (1) Transactional{" "}
          <strong>over-the-air updates with rollback support</strong> of the
          entire system, including firmware files, and (2){" "}
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
        <HomepageFeatures />
      </main>
    </Layout>
  )
}
