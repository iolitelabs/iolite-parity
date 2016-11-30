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

import { combineReducers } from 'redux';
import { routerReducer } from 'react-router-redux';

import { apiReducer, balancesReducer, blockchainReducer, compilerReducer, imagesReducer, personalReducer, signerReducer, statusReducer as nodeStatusReducer, snackbarReducer } from './providers';

import errorReducer from '../ui/Errors/reducers';
import settingsReducer from '../views/Settings/reducers';
import tooltipReducer from '../ui/Tooltips/reducers';

export default function () {
  return combineReducers({
    api: apiReducer,
    errors: errorReducer,
    tooltip: tooltipReducer,
    routing: routerReducer,
    settings: settingsReducer,

    balances: balancesReducer,
    blockchain: blockchainReducer,
    compiler: compilerReducer,
    images: imagesReducer,
    nodeStatus: nodeStatusReducer,
    personal: personalReducer,
    signer: signerReducer,
    snackbar: snackbarReducer
  });
}
