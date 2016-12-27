// Copyright 2015, 2016 Parity Technologies (UK) Ltd.
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

import { sha3 } from '../parity.js';

export const clear = () => ({ type: 'lookup clear' });

export const lookupStart = (name, key) => ({ type: 'lookup start', name, key });
export const reverseLookupStart = (address) => ({ type: 'reverseLookup start', address });

export const success = (action, result) => ({ type: `${action} success`, result: result });

export const fail = (action) => ({ type: `${action} error` });

export const lookup = (name, key) => (dispatch, getState) => {
  const { contract } = getState();
  if (!contract) {
    return;
  }

  const getAddress = contract.functions
    .find((f) => f.name === 'getAddress');

  name = name.toLowerCase();
  dispatch(lookupStart(name, key));

  getAddress.call({}, [ sha3(name), key ])
    .then((address) => dispatch(success('lookup', address)))
    .catch((err) => {
      console.error(`could not lookup ${key} for ${name}`);
      if (err) {
        console.error(err.stack);
      }
      dispatch(fail('lookup'));
    });
};

export const reverseLookup = (address) => (dispatch, getState) => {
  const { contract } = getState();
  if (!contract) {
    return;
  }

  const reverse = contract.functions
    .find((f) => f.name === 'reverse');

  dispatch(reverseLookupStart(address));

  reverse.call({}, [ address ])
    .then((address) => dispatch(success('reverseLookup', address)))
    .catch((err) => {
      console.error(`could not lookup reverse for ${address}`);
      if (err) {
        console.error(err.stack);
      }
      dispatch(fail('reverseLookup'));
    });
};
