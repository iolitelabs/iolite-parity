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

import { sha3, api } from '../parity.js';
import postTx from '../util/post-tx';

export const start = (name, key, value) => ({ type: 'records update start', name, key, value });

export const success = () => ({ type: 'records update success' });

export const fail = () => ({ type: 'records update error' });

export const update = (name, key, value) => (dispatch, getState) => {
  const state = getState();
  const account = state.accounts.selected;
  const contract = state.contract;
  if (!contract || !account) {
    return;
  }

  name = name.toLowerCase();

  const fnName = key === 'A' ? 'setAddress' : 'set';
  const setAddress = contract.functions.find((f) => f.name === fnName);

  dispatch(start(name, key, value));

  const options = {
    from: account.address
  };
  const values = [
    sha3(name),
    key,
    value
  ];

  postTx(api, setAddress, options, values)
    .then((txHash) => {
      dispatch(success());
    }).catch((err) => {
      console.error(`could not update ${key} record of ${name}`);

      if (err) {
        console.error(err.stack);
      }

      dispatch(fail());
    });
};
