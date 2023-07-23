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
          <Admonition type="caution" title="🚧 EXPERIMENTAL 🚧">
            <p>
              Rugpi <strong>is still experimental</strong>. Expect things to
              change and break!
            </p>
            <p>
              If you have any ideas, suggestions, feedback regarding the early
              prototype, or anything else you like to discuss, please reach out
              to us by starting a{" "}
              <a
                href="https://github.com/silitics/rugpi/discussions"
                target="_blank"
              >
                discussion on GitHub
              </a>
              .
            </p>
          </Admonition>
        </div>
        <HomepageFeatures />
      </main>
    </Layout>
  )
}
