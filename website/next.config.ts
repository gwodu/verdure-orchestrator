import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // @verdure/decider ships TypeScript source (the single source of truth shared
  // with the desktop porter), so Next must transpile it rather than expect a
  // pre-built dist. See knowledge/17-build-plan.md.
  transpilePackages: ["@verdure/decider"],

  // The desktop updater's endpoint (knowledge/23).
  //
  // Why it points here and not straight at GitHub: this URL is COMPILED INTO
  // every copy of the app we ship. A binary asks exactly this address for the
  // rest of its life, so whatever we bake in we are married to. Pointing at a
  // GitHub URL would mean that moving hosting — ever, for any reason — silently
  // strands every existing install on a dead address, fixable only by asking
  // each person to download Verdure again by hand.
  //
  // Owning the address costs one redirect and buys the freedom to move the
  // files whenever we like. A redirect (not a rewrite) so GitHub serves the
  // bytes and we don't pay for the bandwidth; `permanent: false` because the
  // whole point is that the destination is expected to change.
  async redirects() {
    return [
      {
        source: "/updates/:file",
        destination:
          "https://github.com/gwodu/Verdure-Releases/releases/latest/download/:file",
        permanent: false,
      },
    ];
  },
};

export default nextConfig;
