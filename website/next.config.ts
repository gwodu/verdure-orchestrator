import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // @verdure/decider ships TypeScript source (the single source of truth shared
  // with the desktop porter), so Next must transpile it rather than expect a
  // pre-built dist. See knowledge/17-build-plan.md.
  transpilePackages: ["@verdure/decider"],
};

export default nextConfig;
