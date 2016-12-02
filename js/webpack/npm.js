// Copyright 2015, 2016 Ethcore (UK) Ltd.
// This file is part of Parity.

// Parity is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity.  If not, see <http://www.gnu.org/licenses/>.

const path = require('path');
const CopyWebpackPlugin = require('copy-webpack-plugin');
const packageJson = require('../package.json');

const Shared = require('./shared');

const ENV = process.env.NODE_ENV || 'development';
const isProd = ENV === 'production';

module.exports = {
  context: path.join(__dirname, '../src'),
  target: 'node',
  entry: 'library.js',
  output: {
    path: path.join(__dirname, '../.npmjs'),
    filename: 'library.js',
    library: 'Parity',
    libraryTarget: 'umd',
    umdNamedDefine: true
  },
  externals: {
    'node-fetch': 'node-fetch',
    'vertx': 'vertx'
  },
  module: {
    noParse: [
      /babel-polyfill/
    ],
    rules: [
      {
        test: /(\.jsx|\.js)$/,
        // use: [ 'happypack/loader?id=js' ],
        use: isProd ? ['babel-loader'] : [
          // 'react-hot-loader',
          'babel-loader?cacheDirectory=true'
        ],
        exclude: /node_modules/
      }
    ]
  },

  resolve: {
    modules: [
      path.resolve('./src'),
      path.join(__dirname, '../node_modules')
    ],
    extensions: ['.json', '.js', '.jsx']
  },

  plugins: Shared.getPlugins().concat([
    new CopyWebpackPlugin([
      {
        from: '../parity.package.json',
        to: 'package.json',
        transform: function (content, path) {
          const json = JSON.parse(content.toString());
          json.version = packageJson.version;
          return new Buffer(JSON.stringify(json, null, '  '), 'utf-8');
        }
      },
      {
        from: '../LICENSE'
      },
      {
        from: '../parity.md',
        to: 'README.md'
      }
    ], { copyUnmodified: true })
  ])
};
