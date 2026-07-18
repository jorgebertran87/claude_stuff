export default {
  paths: ["features/**/*.feature"],
  require: ["tests/steps/**/*.steps.ts"],
  format: ["progress-bar"],
  parallel: 0,
  timeout: 30000,
};
