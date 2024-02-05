module.exports = {
  root: true,
  extends: ["plugin:prettier/recommended"],
  ignorePatterns: [
    'node_modules', // Self-explanatory.
    'out', // Don't lint built library.
    "kanata",
    "kanata-local",
  ],
  overrides: [
    {
      files: ['**/*.ts', '**/*.tsx'],
      extends: [
        'plugin:@typescript-eslint/recommended',
        'plugin:@typescript-eslint/recommended-requiring-type-checking',
      ],
      parserOptions: {
        project: true,
      },
    },
  ],
};
